//! 프론트엔드와 공유하는 DTO 타입.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 인증 방식. 현재는 SQL Server 인증만 지원 (macOS 호환).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    SqlServer,
}

impl Default for AuthMethod {
    fn default() -> Self {
        AuthMethod::SqlServer
    }
}

/// 저장되는 연결 프로필. 비밀번호는 keyring 에 분리 저장.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    #[serde(default)]
    pub auth_method: AuthMethod,
    #[serde(default)]
    pub trust_server_certificate: bool,
    #[serde(default)]
    pub application_name: Option<String>,
}

impl Profile {
    pub fn keyring_account(&self) -> String {
        self.id.to_string()
    }
}

/// 신규 프로필 생성 / 기존 프로필 업데이트시 프론트가 보내는 입력.
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileInput {
    pub id: Option<Uuid>,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    #[serde(default)]
    pub auth_method: AuthMethod,
    #[serde(default)]
    pub trust_server_certificate: bool,
    #[serde(default)]
    pub application_name: Option<String>,
}

impl ProfileInput {
    pub fn into_profile_with_id(self, id: Uuid) -> Profile {
        Profile {
            id,
            name: self.name,
            host: self.host,
            port: self.port,
            database: self.database,
            username: self.username,
            auth_method: self.auth_method,
            trust_server_certificate: self.trust_server_certificate,
            application_name: self.application_name,
        }
    }
}

/// profiles.json 의 루트 구조.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProfileStore {
    pub profiles: Vec<Profile>,
}
