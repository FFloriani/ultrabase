mod browser;
mod bundle;
mod commands;
mod db;
mod error;
mod models;
mod session;
mod slots;
mod supabase;
mod sync;
mod vault;

use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use tauri::{Manager, WindowEvent};

use crate::error::{AppError, AppResult};

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub http: reqwest::Client,
    /// Contas cujo navegador está sendo vigiado à espera do login.
    pub watching: Mutex<HashSet<String>>,
}

impl AppState {
    pub fn conn(&self) -> AppResult<MutexGuard<'_, rusqlite::Connection>> {
        self.db.lock().map_err(|_| AppError::Invalid("o banco local travou; feche e abra o app".into()))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open(&data_dir.join("ultrabase.db"))?;
            browser::cleanup_orphan_profiles(&browser::profiles_root(app.handle())?, &db::account_ids(&conn)?);
            let http = reqwest::Client::builder()
                .user_agent(concat!("ultrabase/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(20))
                .build()?;
            app.manage(AppState { db: Mutex::new(conn), http, watching: Mutex::new(HashSet::new()) });
            Ok(())
        })
        // Fechar a janela principal fecha o app inteiro, junto com as janelas das contas.
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { .. } = event {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::start_account,
            commands::update_account,
            commands::delete_account,
            commands::reveal_secret,
            commands::sync_account,
            commands::sync_all,
            commands::open_browser,
            commands::project_action,
            commands::project_secrets,
            commands::set_db_password,
            commands::reset_db_password,
            commands::create_ai_token,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Ultrabase");
}
