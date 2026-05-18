//! 오브젝트 익스플로러 — 읽기 전용 카탈로그 조회.
//!
//! 노드 종류:
//!  - 서버(루트): 그 아래 데이터베이스 목록
//!  - 데이터베이스: 그 아래 Tables / Views / StoredProcedures 그룹
//!  - 그룹 안에 실제 객체들
//!
//! 모든 쿼리는 sys 카탈로그 뷰만 사용한다 → 권한이 낮은 read-only 계정에서도 동작.
//! DB 이름은 sys.databases 에서 직접 받은 것 + `QUOTENAME` 으로 escape 해서 인젝션 차단.

use crate::error::{AppError, AppResult};
use crate::session::{Session, SessionManager};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use tauri::State;
use tiberius::QueryItem;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Table,
    View,
    Procedure,
}

#[derive(Debug, Serialize)]
pub struct DbObject {
    pub schema: String,
    pub name: String,
}

async fn fetch_databases(session: &Session) -> AppResult<Vec<String>> {
    let mut conn = session
        .pool
        .get()
        .await
        .map_err(|e| AppError::Connect(format!("커넥션 확보 실패: {e}")))?;
    // 시스템 DB(master/msdb/tempdb) 는 익스플로러에서 제외.
    let sql = "
        SELECT name FROM sys.databases
        WHERE state_desc = 'ONLINE' AND HAS_DBACCESS(name) = 1
          AND name NOT IN ('master', 'msdb', 'tempdb')
        ORDER BY name
    ";
    let mut stream = conn.simple_query(sql).await?;
    let mut out: Vec<String> = Vec::new();
    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            if let Ok(Some(name)) = row.try_get::<&str, _>(0) {
                out.push(name.to_string());
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn list_databases(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
) -> AppResult<Vec<String>> {
    let session = sessions.get(session_id).await?;
    fetch_databases(&session).await
}

#[tauri::command]
pub async fn list_objects(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
    database: String,
    kind: ObjectKind,
) -> AppResult<Vec<DbObject>> {
    let session = sessions.get(session_id).await?;

    // sys.databases 에서 받은 이름인지 1차 검증
    let dbs = fetch_databases(&session).await?;
    if !dbs.iter().any(|n| n == &database) {
        return Err(AppError::ConfigInvalid(format!(
            "알 수 없는 데이터베이스: {database}"
        )));
    }

    let view = match kind {
        ObjectKind::Table => "sys.tables",
        ObjectKind::View => "sys.views",
        ObjectKind::Procedure => "sys.procedures",
    };

    // QUOTENAME 형식의 식별자 escape (`]` 를 `]]` 로) — 2차 방어
    let escaped = database.replace(']', "]]");
    let sql = format!(
        "SELECT s.name AS schema_name, o.name AS object_name \
         FROM [{escaped}].{view} o \
         JOIN [{escaped}].sys.schemas s ON o.schema_id = s.schema_id \
         ORDER BY s.name, o.name"
    );

    let mut conn = session
        .pool
        .get()
        .await
        .map_err(|e| AppError::Connect(format!("커넥션 확보 실패: {e}")))?;
    let mut stream = conn.simple_query(sql).await?;
    let mut out: Vec<DbObject> = Vec::new();
    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            let schema = row
                .try_get::<&str, _>(0)
                .ok()
                .flatten()
                .map(|s| s.to_string());
            let name = row
                .try_get::<&str, _>(1)
                .ok()
                .flatten()
                .map(|s| s.to_string());
            if let (Some(schema), Some(name)) = (schema, name) {
                out.push(DbObject { schema, name });
            }
        }
    }
    Ok(out)
}
