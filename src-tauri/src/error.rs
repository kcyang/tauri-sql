//! 앱 전체 에러 타입.
//!
//! 모든 `#[tauri::command]` 는 `Result<T, AppError>` 를 반환한다.
//! AppError 는 `Serialize` 를 직접 구현해 프론트엔드에 `{ kind, message, detail? }` 형태로 전달된다.

use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)] // 일부 variant 는 Phase 2 이후에 생성됨
pub enum AppError {
    #[error("설정이 올바르지 않습니다: {0}")]
    ConfigInvalid(String),

    #[error("연결 실패: {0}")]
    Connect(String),

    #[error("인증 실패")]
    Auth,

    #[error("쿼리 시간 초과")]
    Timeout,

    #[error("쿼리가 취소되었습니다")]
    Cancelled,

    #[error("SQL 오류: {0}")]
    Sql(String),

    #[error("자격 증명 저장소 오류: {0}")]
    Keyring(String),

    #[error("입출력 오류: {0}")]
    Io(String),

    #[error("세션을 찾을 수 없습니다")]
    SessionNotFound,

    #[error("내부 오류: {0}")]
    Internal(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let kind = match self {
            AppError::ConfigInvalid(_) => "config_invalid",
            AppError::Connect(_) => "connect",
            AppError::Auth => "auth",
            AppError::Timeout => "timeout",
            AppError::Cancelled => "cancelled",
            AppError::Sql(_) => "sql",
            AppError::Keyring(_) => "keyring",
            AppError::Io(_) => "io",
            AppError::SessionNotFound => "session_not_found",
            AppError::Internal(_) => "internal",
        };
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", kind)?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Internal(format!("JSON: {e}"))
    }
}

impl From<keyring_core::Error> for AppError {
    fn from(e: keyring_core::Error) -> Self {
        AppError::Keyring(e.to_string())
    }
}

impl From<tiberius::error::Error> for AppError {
    fn from(e: tiberius::error::Error) -> Self {
        use tiberius::error::Error as TE;
        match e {
            TE::Server(t) => AppError::Sql(format!("[{}] {}", t.code(), t.message())),
            TE::Io { kind: _, message } => AppError::Connect(message),
            TE::Tls(msg) => AppError::Connect(format!("TLS: {msg}")),
            other => AppError::Connect(other.to_string()),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
