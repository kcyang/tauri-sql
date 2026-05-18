//! 연결 프로필 영구 저장 + 비밀번호 keyring 연동.
//!
//! - 프로필 JSON: `<app_data_dir>/profiles.json`
//! - 비밀번호: keyring (`service="tauri-sql"`, `account=<profile.id>`)
//!
//! 동시 접근 보호: `parking_lot::Mutex` 로 파일 I/O 직렬화.

use crate::error::{AppError, AppResult};
use crate::types::{Profile, ProfileInput, ProfileStore};
use parking_lot::Mutex;
use std::path::PathBuf;
use uuid::Uuid;

const KEYRING_SERVICE: &str = "tauri-sql";

/// 프로필 저장소. Tauri State 로 등록.
pub struct ProfileRepository {
    path: PathBuf,
    lock: Mutex<()>,
}

impl ProfileRepository {
    pub fn new(app_data_dir: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&app_data_dir)?;
        Ok(Self {
            path: app_data_dir.join("profiles.json"),
            lock: Mutex::new(()),
        })
    }

    fn load_locked(&self) -> AppResult<ProfileStore> {
        if !self.path.exists() {
            return Ok(ProfileStore::default());
        }
        let bytes = std::fs::read(&self.path)?;
        if bytes.is_empty() {
            return Ok(ProfileStore::default());
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save_locked(&self, store: &ProfileStore) -> AppResult<()> {
        let tmp = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(store)?;
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &self.path)?; // 원자적 교체
        Ok(())
    }

    pub fn list(&self) -> AppResult<Vec<Profile>> {
        let _g = self.lock.lock();
        Ok(self.load_locked()?.profiles)
    }

    /// id 가 None 이면 신규 발급, Some 이면 해당 프로필을 업데이트.
    /// password 가 Some 이면 keyring 에 저장 (None 이면 기존 비밀번호 유지).
    pub fn save(&self, input: ProfileInput, password: Option<String>) -> AppResult<Profile> {
        let _g = self.lock.lock();
        let mut store = self.load_locked()?;
        let id = input.id.unwrap_or_else(Uuid::new_v4);
        let profile = input.into_profile_with_id(id);

        if let Some(idx) = store.profiles.iter().position(|p| p.id == id) {
            store.profiles[idx] = profile.clone();
        } else {
            store.profiles.push(profile.clone());
        }
        self.save_locked(&store)?;

        if let Some(pw) = password {
            set_password(&profile, &pw)?;
        }
        Ok(profile)
    }

    pub fn delete(&self, id: Uuid) -> AppResult<()> {
        let _g = self.lock.lock();
        let mut store = self.load_locked()?;
        let removed = store
            .profiles
            .iter()
            .position(|p| p.id == id)
            .map(|idx| store.profiles.remove(idx));
        self.save_locked(&store)?;

        if let Some(profile) = removed {
            // keyring 삭제 실패는 무시 (이미 없을 수 있음)
            let _ = delete_password(&profile);
        }
        Ok(())
    }

    pub fn get(&self, id: Uuid) -> AppResult<Profile> {
        let _g = self.lock.lock();
        self.load_locked()?
            .profiles
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::ConfigInvalid("프로필을 찾을 수 없습니다".into()))
    }
}

pub fn set_password(profile: &Profile, password: &str) -> AppResult<()> {
    let entry = keyring_core::Entry::new(KEYRING_SERVICE, &profile.keyring_account())?;
    entry.set_password(password)?;
    Ok(())
}

pub fn get_password(profile: &Profile) -> AppResult<Option<String>> {
    let entry = keyring_core::Entry::new(KEYRING_SERVICE, &profile.keyring_account())?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

pub fn delete_password(profile: &Profile) -> AppResult<()> {
    let entry = keyring_core::Entry::new(KEYRING_SERVICE, &profile.keyring_account())?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring_core::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

