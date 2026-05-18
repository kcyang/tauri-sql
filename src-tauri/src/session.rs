//! 세션 매니저.
//!
//! 세션 = 하나의 연결 프로필에 대해 활성화된 bb8 풀. UUID 키로 보관.
//! Tauri State 로 등록되어 어디서든 접근 가능.

use crate::error::{AppError, AppResult};
use crate::pool::{build_pool, TiberiusPool};
use crate::types::Profile;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct Session {
    #[allow(dead_code)] // id 는 디버깅/로깅 용도, 외부에서 키로 별도 관리
    pub id: Uuid,
    pub profile: Profile,
    pub pool: TiberiusPool,
}

#[derive(Default)]
pub struct SessionManager {
    inner: Arc<RwLock<HashMap<Uuid, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn open(&self, profile: Profile, password: &str) -> AppResult<Uuid> {
        let pool = build_pool(&profile, password).await?;
        // 풀이 살아있는지 즉시 검증 — SELECT 1
        {
            let mut conn = pool
                .get()
                .await
                .map_err(|e| AppError::Connect(format!("초기 커넥션 확보 실패: {e}")))?;
            let _ = conn.simple_query("SELECT 1").await?.into_row().await?;
        }
        let id = Uuid::new_v4();
        let session = Session {
            id,
            profile,
            pool,
        };
        self.inner.write().await.insert(id, session);
        Ok(id)
    }

    pub async fn close(&self, id: Uuid) -> AppResult<()> {
        self.inner.write().await.remove(&id);
        Ok(()) // 풀은 drop 시 자동 정리
    }

    /// 앱 종료 시 모든 세션을 한 번에 정리.
    /// inner map 에서 Session 들을 제거하면 풀(bb8::Pool) 의 마지막 참조가 사라지고
    /// 내부 tiberius Client 들이 drop → TCP socket close 가 결정론적으로 일어난다.
    pub async fn close_all(&self) -> usize {
        let mut guard = self.inner.write().await;
        let n = guard.len();
        guard.clear();
        n
    }

    pub async fn get(&self, id: Uuid) -> AppResult<Session> {
        self.inner
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or(AppError::SessionNotFound)
    }
}
