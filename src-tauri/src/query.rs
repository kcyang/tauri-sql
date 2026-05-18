//! 쿼리 실행 + 취소 + 타임아웃 + max_rows.
//!
//! tiberius 는 네이티브 cancel 을 지원하지 않으므로 "취소 = 그 커넥션을 무효화"로 구현.
//! 동작 흐름:
//!   1) 프론트가 query_id(UUID)를 발급해 호출
//!   2) 백엔드는 그 query_id 에 `CancellationToken` 을 등록
//!   3) `tokio::select!` 로 (쿼리, 타임아웃, 취소) 경쟁
//!   4) 어느 분기든 종료되면 select! 가 나머지를 drop → 풀로 돌아간 커넥션은
//!      `test_on_check_out` 으로 다음 사용 시 자동 검증/폐기
//!   5) 등록된 토큰 제거

use crate::error::{AppError, AppResult};
use crate::session::SessionManager;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use futures_util::TryStreamExt;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::State;
use tiberius::{ColumnType, QueryItem};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const DEFAULT_MAX_ROWS: usize = 1000;
const DEFAULT_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum RowValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Decimal(String),
    Text(String),
    DateTime(String),
    Uuid(String),
    Binary(String),
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnMeta {
    pub name: String,
    pub sql_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Vec<RowValue>>,
    pub row_count: u64,
    pub truncated: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteQueryArgs {
    pub session_id: Uuid,
    pub sql: String,
    pub query_id: Uuid,
    pub max_rows: Option<usize>,
    pub timeout_ms: Option<u64>,
}

/// 진행 중인 쿼리 → CancellationToken.
/// Tauri State 로 등록.
#[derive(Default)]
pub struct QueryRegistry {
    inner: Arc<Mutex<HashMap<Uuid, CancellationToken>>>,
}

impl QueryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn register(&self, id: Uuid) -> CancellationToken {
        let token = CancellationToken::new();
        self.inner.lock().insert(id, token.clone());
        token
    }
    fn unregister(&self, id: Uuid) {
        self.inner.lock().remove(&id);
    }
    fn cancel(&self, id: Uuid) -> bool {
        if let Some(token) = self.inner.lock().get(&id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// 앱 종료 시 모든 진행 중인 쿼리에 취소 신호.
    /// 각 execute_query 의 `tokio::select!` 가 cancel branch 로 종료되며
    /// 보유한 PooledConnection 이 풀로 반환된다 (이후 pool drop 시 close).
    pub fn cancel_all(&self) -> usize {
        let guard = self.inner.lock();
        let n = guard.len();
        for (_, token) in guard.iter() {
            token.cancel();
        }
        n
    }
}

fn classify_column(ct: ColumnType) -> &'static str {
    use ColumnType::*;
    match ct {
        Null => "null",
        Bit | Bitn => "bool",
        Int1 | Int2 | Int4 | Int8 | Intn => "int",
        Float4 | Float8 | Floatn => "float",
        Money | Money4 => "money",
        Numericn | Decimaln => "decimal",
        Datetime | Datetime2 | Datetime4 | Datetimen | DatetimeOffsetn => "datetime",
        Daten => "date",
        Timen => "time",
        Guid => "uuid",
        NVarchar | Text | NText | BigVarChar | BigChar | NChar => "text",
        BigBinary | BigVarBin | Image | Udt => "binary",
        Xml => "xml",
        SSVariant => "variant",
    }
}

fn extract_value(row: &tiberius::Row, idx: usize, ct: ColumnType) -> RowValue {
    use ColumnType::*;
    match ct {
        Null => RowValue::Null,
        Bit | Bitn => row
            .try_get::<bool, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, RowValue::Bool),
        Int1 => row
            .try_get::<u8, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |v| RowValue::Int(v as i64)),
        Int2 => row
            .try_get::<i16, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |v| RowValue::Int(v as i64)),
        Int4 => row
            .try_get::<i32, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |v| RowValue::Int(v as i64)),
        Int8 => row
            .try_get::<i64, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, RowValue::Int),
        Intn => {
            if let Ok(Some(v)) = row.try_get::<i64, _>(idx) {
                RowValue::Int(v)
            } else if let Ok(Some(v)) = row.try_get::<i32, _>(idx) {
                RowValue::Int(v as i64)
            } else if let Ok(Some(v)) = row.try_get::<i16, _>(idx) {
                RowValue::Int(v as i64)
            } else if let Ok(Some(v)) = row.try_get::<u8, _>(idx) {
                RowValue::Int(v as i64)
            } else {
                RowValue::Null
            }
        }
        Float4 => row
            .try_get::<f32, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |v| RowValue::Float(v as f64)),
        Float8 | Floatn => row
            .try_get::<f64, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, RowValue::Float),
        Money | Money4 | Numericn | Decimaln => row
            .try_get::<rust_decimal::Decimal, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |v| RowValue::Decimal(v.to_string())),
        NVarchar | Text | NText | BigVarChar | BigChar | NChar => row
            .try_get::<&str, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |s| RowValue::Text(s.to_string())),
        Guid => row
            .try_get::<tiberius::Uuid, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |u| RowValue::Uuid(u.to_string())),
        BigBinary | BigVarBin | Image | Udt => row
            .try_get::<&[u8], _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |b| RowValue::Binary(B64.encode(b))),
        Datetime | Datetime2 | Datetime4 | Datetimen => row
            .try_get::<NaiveDateTime, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |dt| {
                RowValue::DateTime(dt.format("%Y-%m-%dT%H:%M:%S%.f").to_string())
            }),
        DatetimeOffsetn => row
            .try_get::<DateTime<FixedOffset>, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |dt| RowValue::DateTime(dt.to_rfc3339())),
        Daten => row
            .try_get::<NaiveDate, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |d| RowValue::DateTime(d.to_string())),
        Timen => row
            .try_get::<NaiveTime, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |t| {
                RowValue::DateTime(t.format("%H:%M:%S%.f").to_string())
            }),
        Xml => row
            .try_get::<&str, _>(idx)
            .ok()
            .flatten()
            .map_or(RowValue::Null, |s| RowValue::Text(s.to_string())),
        SSVariant => RowValue::Unknown("variant".into()),
    }
}

async fn run_query(
    sessions: &SessionManager,
    args: &ExecuteQueryArgs,
    max_rows: usize,
) -> AppResult<QueryResult> {
    let session = sessions.get(args.session_id).await?;
    let started = Instant::now();
    let mut conn = session
        .pool
        .get()
        .await
        .map_err(|e| AppError::Connect(format!("커넥션 확보 실패: {e}")))?;
    let mut stream = conn.simple_query(args.sql.clone()).await?;

    let mut columns: Vec<ColumnMeta> = Vec::new();
    let mut rows: Vec<Vec<RowValue>> = Vec::new();
    let mut truncated = false;
    let mut total_rows: u64 = 0;

    while let Some(item) = stream.try_next().await? {
        match item {
            QueryItem::Metadata(meta) => {
                if columns.is_empty() && meta.result_index() == 0 {
                    columns = meta
                        .columns()
                        .iter()
                        .map(|c| ColumnMeta {
                            name: c.name().to_string(),
                            sql_type: classify_column(c.column_type()).into(),
                        })
                        .collect();
                }
            }
            QueryItem::Row(row) => {
                if row.result_index() != 0 {
                    continue;
                }
                total_rows += 1;
                if rows.len() < max_rows {
                    let values: Vec<RowValue> = row
                        .columns()
                        .iter()
                        .enumerate()
                        .map(|(i, c)| extract_value(&row, i, c.column_type()))
                        .collect();
                    rows.push(values);
                } else {
                    truncated = true;
                    break;
                }
            }
        }
    }

    let elapsed_ms = started.elapsed().as_millis() as u64;
    let row_count = rows.len() as u64;
    Ok(QueryResult {
        columns,
        rows,
        row_count: if truncated { row_count } else { total_rows },
        truncated,
        elapsed_ms,
    })
}

#[tauri::command]
pub async fn execute_query(
    sessions: State<'_, SessionManager>,
    registry: State<'_, QueryRegistry>,
    args: ExecuteQueryArgs,
) -> AppResult<QueryResult> {
    let query_id = args.query_id;
    let max_rows = args.max_rows.unwrap_or(DEFAULT_MAX_ROWS);
    let timeout = Duration::from_millis(args.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
    let token = registry.register(query_id);

    let outcome = tokio::select! {
        biased; // 취소/타임아웃을 먼저 평가
        _ = token.cancelled() => Err(AppError::Cancelled),
        _ = tokio::time::sleep(timeout) => Err(AppError::Timeout),
        r = run_query(&sessions, &args, max_rows) => r,
    };

    registry.unregister(query_id);
    outcome
}

#[tauri::command]
pub async fn cancel_query(registry: State<'_, QueryRegistry>, query_id: Uuid) -> AppResult<bool> {
    Ok(registry.cancel(query_id))
}

#[tauri::command]
pub fn classify_sql(sql: String) -> crate::sql_safety::SqlClassification {
    crate::sql_safety::classify(&sql)
}
