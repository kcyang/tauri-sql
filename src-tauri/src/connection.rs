//! tiberius `Config` 빌드와 테스트 연결.
//!
//! 풀(pool.rs)과 일회성 연결 테스트가 공유하는 저수준 헬퍼.

use crate::error::{AppError, AppResult};
use crate::types::{AuthMethod, Profile};
use tiberius::{AuthMethod as TAuth, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

/// 프로필 + 비밀번호 → tiberius Config.
pub fn build_config(profile: &Profile, password: &str) -> Config {
    let mut config = Config::new();
    config.host(&profile.host);
    config.port(profile.port);
    config.database(&profile.database);

    if profile.trust_server_certificate {
        config.trust_cert();
    }
    if let Some(app) = &profile.application_name {
        config.application_name(app);
    }

    match profile.auth_method {
        AuthMethod::SqlServer => {
            config.authentication(TAuth::sql_server(&profile.username, password));
        }
    }

    config
}

/// 일회성 연결을 만들고 `SELECT 1` 로 동작을 확인한다.
/// 풀에 등록하지 않으므로 함수 종료 시 커넥션이 닫힌다.
pub async fn test_connect(profile: &Profile, password: &str) -> AppResult<()> {
    let config = build_config(profile, password);
    let tcp = TcpStream::connect(config.get_addr())
        .await
        .map_err(|e| AppError::Connect(format!("TCP: {e}")))?;
    tcp.set_nodelay(true)
        .map_err(|e| AppError::Connect(format!("TCP nodelay: {e}")))?;

    let mut client = Client::connect(config, tcp.compat_write()).await?;
    let row = client.query("SELECT 1", &[]).await?.into_row().await?;
    if row.is_none() {
        return Err(AppError::Connect("SELECT 1 응답 없음".into()));
    }
    let _ = client.close().await;
    Ok(())
}
