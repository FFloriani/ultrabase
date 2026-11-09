//! Um navegador por conta. Cada janela usa a própria pasta de dados do WebView2, então login,
//! cookies e localStorage de uma conta não aparecem nas outras, e a sessão continua depois de
//! fechar o app. Essas janelas carregam sites de fora e não têm permissão para chamar o app
//! (a permissão do app vale só para a janela "main", em capabilities/default.json).

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{
    webview::NewWindowResponse, AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

use crate::error::{AppError, AppResult};
use crate::models::Account;

const DASHBOARD: &str = "https://supabase.com/dashboard";
pub const EVENT: &str = "browser-changed";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserEvent {
    account_id: String,
    open: bool,
}

pub fn is_valid_ref(project_ref: &str) -> bool {
    !project_ref.is_empty() && project_ref.len() <= 64 && project_ref.chars().all(|c| c.is_ascii_alphanumeric())
}

pub fn target_url(target: &str, project_ref: Option<&str>) -> AppResult<Url> {
    let url = match (target, project_ref) {
        ("dashboard", _) => format!("{DASHBOARD}/projects"),
        ("tokens", _) => format!("{DASHBOARD}/account/tokens"),
        ("signin", _) => format!("{DASHBOARD}/sign-in"),
        ("signup", _) => format!("{DASHBOARD}/sign-up"),
        ("project", Some(r)) if is_valid_ref(r) => format!("{DASHBOARD}/project/{r}"),
        _ => return Err(AppError::Invalid(format!("destino desconhecido: {target}"))),
    };
    url.parse().map_err(|e| AppError::Invalid(format!("endereço inválido: {e}")))
}

fn window_label(account_id: &str) -> String {
    format!("conta-{account_id}")
}

pub fn profiles_root(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_data_dir()?.join("perfis"))
}

pub fn window(app: &AppHandle, account_id: &str) -> Option<WebviewWindow> {
    app.get_webview_window(&window_label(account_id))
}

/// Aberto = janela que o dono vê. A janela escondida da sincronização não conta.
pub fn is_open(app: &AppHandle, account_id: &str) -> bool {
    window(app, account_id).is_some_and(|w| w.is_visible().unwrap_or(false))
}

fn emit(app: &AppHandle, account_id: &str, open: bool) {
    let _ = app.emit(EVENT, BrowserEvent { account_id: account_id.to_string(), open });
}

fn build(app: &AppHandle, account: &Account, url: Url, visible: bool) -> AppResult<WebviewWindow> {
    let data_dir = profiles_root(app)?.join(&account.id);
    std::fs::create_dir_all(&data_dir)?;
    let window = WebviewWindowBuilder::new(app, window_label(&account.id), WebviewUrl::External(url))
        .title(format!("{} · Ultrabase", account.alias))
        .inner_size(1280.0, 860.0)
        .min_inner_size(800.0, 560.0)
        .visible(visible)
        .data_directory(data_dir)
        // Pop-ups (login pelo GitHub, por exemplo) abrem com o mesmo perfil da conta.
        .on_new_window(|_, _| NewWindowResponse::Allow)
        .build()?;

    let handle = app.clone();
    let account_id = account.id.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Destroyed = event {
            emit(&handle, &account_id, false);
        }
    });
    Ok(window)
}

/// Mostra o navegador da conta no endereço pedido; cria a janela se ainda não existir.
pub fn open(app: &AppHandle, account: &Account, url: Url) -> AppResult<WebviewWindow> {
    let window = match window(app, &account.id) {
        Some(window) => {
            window.navigate(url)?;
            window
        }
        None => build(app, account, url, true)?,
    };
    window.show()?;
    let _ = window.unminimize();
    window.set_focus()?;
    emit(app, &account.id, true);
    Ok(window)
}

/// Janela invisível com o perfil da conta, só para o app ler a sessão. Quem pediu fecha depois
/// com `close_if_hidden`.
pub fn open_hidden(app: &AppHandle, account: &Account) -> AppResult<WebviewWindow> {
    build(app, account, target_url("dashboard", None)?, false)
}

/// Fecha a janela só se o dono não estiver usando (ele pode ter clicado em "Abrir navegador" no meio).
pub fn close_if_hidden(window: &WebviewWindow) {
    if !window.is_visible().unwrap_or(false) {
        let _ = window.destroy();
    }
}

pub fn close(app: &AppHandle, account_id: &str) {
    if let Some(window) = window(app, account_id) {
        let _ = window.destroy();
    }
}

/// Apaga a pasta de navegador de contas que não existem mais. Roda ao abrir o app,
/// porque logo depois de fechar a janela o WebView2 ainda pode estar segurando os arquivos.
pub fn cleanup_orphan_profiles(root: &Path, keep_ids: &[String]) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() && !keep_ids.contains(&name) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_for_each_target() {
        assert_eq!(target_url("tokens", None).unwrap().as_str(), "https://supabase.com/dashboard/account/tokens");
        assert_eq!(
            target_url("project", Some("abcdefghijklmnopqrst")).unwrap().as_str(),
            "https://supabase.com/dashboard/project/abcdefghijklmnopqrst"
        );
    }

    #[test]
    fn rejects_bad_targets() {
        assert!(target_url("project", None).is_err());
        assert!(target_url("project", Some("../account")).is_err());
        assert!(target_url("javascript", None).is_err());
    }

    #[test]
    fn cleanup_keeps_only_existing_accounts() {
        let root = std::env::temp_dir().join(format!("ultrabase-teste-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("fica")).unwrap();
        std::fs::create_dir_all(root.join("sai")).unwrap();
        cleanup_orphan_profiles(&root, &["fica".to_string()]);
        assert!(root.join("fica").exists());
        assert!(!root.join("sai").exists());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
