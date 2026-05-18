//! 파일 읽기/쓰기 커맨드.
//!
//! 프론트엔드는 `tauri-plugin-dialog` 으로 사용자에게 경로를 받은 뒤
//! 여기 정의된 커맨드를 호출해 텍스트를 읽거나 쓴다.
//!
//! 파일 시스템 접근 권한은 dialog 플러그인이 사용자 명시 동의를 받으므로,
//! 별도 fs 플러그인 scope 설정 없이 std::fs 로 직접 처리한다.

use crate::error::{AppError, AppResult};

const MAX_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB 안전 한도

#[tauri::command]
pub fn read_text_file(path: String) -> AppResult<String> {
    let meta = std::fs::metadata(&path).map_err(AppError::from)?;
    if !meta.is_file() {
        return Err(AppError::ConfigInvalid("파일이 아닙니다".into()));
    }
    if meta.len() > MAX_SIZE_BYTES {
        return Err(AppError::ConfigInvalid(format!(
            "파일이 너무 큽니다 ({} bytes, 한도 {} bytes)",
            meta.len(),
            MAX_SIZE_BYTES
        )));
    }
    std::fs::read_to_string(&path).map_err(AppError::from)
}

#[tauri::command]
pub fn write_text_file(path: String, content: String) -> AppResult<()> {
    if content.len() as u64 > MAX_SIZE_BYTES {
        return Err(AppError::ConfigInvalid(format!(
            "내용이 너무 큽니다 ({} bytes, 한도 {} bytes)",
            content.len(),
            MAX_SIZE_BYTES
        )));
    }
    std::fs::write(&path, content).map_err(AppError::from)
}
