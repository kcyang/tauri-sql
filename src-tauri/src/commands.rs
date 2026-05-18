//! Tauri 커맨드 정의.
//!
//! Phase 1: 프로필 CRUD + 연결 테스트.
//! 이후 phase 에서 세션/쿼리/익스플로러 커맨드가 추가된다.

use crate::connection;
use crate::error::{AppError, AppResult};
use crate::profiles::{self, ProfileRepository};
use crate::session::SessionManager;
use crate::types::{Profile, ProfileInput};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn list_profiles(repo: State<'_, ProfileRepository>) -> AppResult<Vec<Profile>> {
    repo.list()
}

#[tauri::command]
pub fn save_profile(
    repo: State<'_, ProfileRepository>,
    input: ProfileInput,
    password: Option<String>,
) -> AppResult<Profile> {
    repo.save(input, password)
}

#[tauri::command]
pub fn delete_profile(repo: State<'_, ProfileRepository>, id: Uuid) -> AppResult<()> {
    repo.delete(id)
}

/// 비밀번호 해석:
///  1) 인자로 password 가 들어왔으면 그대로 사용
///  2) input.id 가 저장된 프로필이면 keyring 조회
///  3) 둘 다 없으면 에러
fn resolve_password(
    repo: &ProfileRepository,
    input: &ProfileInput,
    password: Option<String>,
) -> AppResult<String> {
    if let Some(pw) = password {
        return Ok(pw);
    }
    let pid = input
        .id
        .ok_or_else(|| AppError::ConfigInvalid("비밀번호가 필요합니다".into()))?;
    let profile = repo.get(pid)?;
    profiles::get_password(&profile)?
        .ok_or_else(|| AppError::ConfigInvalid("저장된 비밀번호가 없습니다".into()))
}

#[tauri::command]
pub async fn test_connection(
    repo: State<'_, ProfileRepository>,
    input: ProfileInput,
    password: Option<String>,
) -> AppResult<()> {
    let pw = resolve_password(&repo, &input, password)?;
    let id = input.id.unwrap_or_else(Uuid::new_v4);
    let profile = input.into_profile_with_id(id);
    connection::test_connect(&profile, &pw).await
}

/// 저장된 프로필로 세션을 연다. 비밀번호는 keyring 에서 자동 조회.
#[tauri::command]
pub async fn open_session(
    repo: State<'_, ProfileRepository>,
    sessions: State<'_, SessionManager>,
    profile_id: Uuid,
) -> AppResult<Uuid> {
    let profile = repo.get(profile_id)?;
    let pw = profiles::get_password(&profile)?
        .ok_or_else(|| AppError::ConfigInvalid("저장된 비밀번호가 없습니다".into()))?;
    sessions.open(profile, &pw).await
}

#[tauri::command]
pub async fn close_session(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
) -> AppResult<()> {
    sessions.close(session_id).await
}

/// 활성 세션의 프로필 메타 반환 — workspace 헤더 표시용.
#[tauri::command]
pub async fn session_profile(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
) -> AppResult<Profile> {
    Ok(sessions.get(session_id).await?.profile)
}
