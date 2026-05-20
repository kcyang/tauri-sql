//! just-sql — MSSQL 쿼리/탐색 데스크톱 앱.

mod commands;
mod connection;
mod error;
mod explorer;
mod export;
mod file_io;
mod pool;
mod profiles;
mod query;
mod session;
mod sql_safety;
mod types;

use profiles::ProfileRepository;
use query::QueryRegistry;
use session::SessionManager;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // OS 네이티브 자격 증명 저장소 초기화 (macOS: Keychain)
            keyring::use_native_store(false)
                .map_err(|e| format!("keyring 초기화 실패: {e}"))?;

            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("app_data_dir 확인 실패");
            let repo = ProfileRepository::new(app_data_dir)
                .expect("ProfileRepository 초기화 실패");
            app.manage(repo);
            app.manage(SessionManager::new());
            app.manage(QueryRegistry::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::test_connection,
            commands::open_session,
            commands::close_session,
            commands::session_profile,
            query::execute_query,
            query::cancel_query,
            query::classify_sql,
            explorer::list_databases,
            explorer::list_objects,
            file_io::read_text_file,
            file_io::write_text_file,
            export::export_query_xlsx,
        ])
        .build(tauri::generate_context!())
        .expect("Tauri 앱 빌드 실패");

    // 종료 시점에 활성 쿼리 취소 + 모든 세션 graceful close.
    // ExitRequested: 종료 결정 직후, prevent_exit() 호출 안 하면 그대로 진행.
    // Exit: 실제 종료 직전 — 최종 fallback.
    app.run(|app_handle, event| match event {
        tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
            graceful_shutdown(app_handle);
        }
        _ => {}
    });
}

/// 활성 쿼리 취소 → 세션/풀 정리.
/// 3초 타임아웃 — hang 되어도 OS 가 socket 정리하므로 종료 자체는 보장.
/// 이미 한 번 정리된 후 다시 호출되어도 안전 (idempotent).
fn graceful_shutdown(app: &tauri::AppHandle) {
    use std::time::Duration;

    let canceled = match app.try_state::<QueryRegistry>() {
        Some(qr) => qr.cancel_all(),
        None => 0,
    };

    if let Some(sm) = app.try_state::<SessionManager>() {
        let closed = tauri::async_runtime::block_on(async {
            match tokio::time::timeout(Duration::from_secs(3), sm.close_all()).await {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("[shutdown] 세션 close 타임아웃 (3s) — 강제 종료 진행");
                    0
                }
            }
        });
        if canceled > 0 || closed > 0 {
            eprintln!(
                "[shutdown] 쿼리 취소: {canceled}건, 세션 정리: {closed}건"
            );
        }
    }
}
