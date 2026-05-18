//! bb8-tiberius 커넥션 풀 구성.
//!
//! 풀 크기 4 — 동시 쿼리 최대 4건, 익스플로러 + 일반 쿼리 분리.
//! 연결 단계에서 타임아웃 10초.

use crate::connection;
use crate::error::{AppError, AppResult};
use crate::types::Profile;
use bb8::Pool;
use bb8_tiberius::ConnectionManager;
use std::time::Duration;

pub type TiberiusPool = Pool<ConnectionManager>;

pub async fn build_pool(profile: &Profile, password: &str) -> AppResult<TiberiusPool> {
    let config = connection::build_config(profile, password);
    let manager = ConnectionManager::new(config);
    Pool::builder()
        .max_size(4)
        .connection_timeout(Duration::from_secs(10))
        // 취소된 쿼리가 반납한 손상 가능성 있는 커넥션을 자동 폐기하기 위해 check-out 시점에 검증
        .test_on_check_out(true)
        .build(manager)
        .await
        .map_err(|e| AppError::Connect(format!("풀 생성 실패: {e}")))
}
