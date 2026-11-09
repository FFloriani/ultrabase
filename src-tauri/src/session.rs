//! Sessão do painel do Supabase guardada no navegador de cada conta.
//! O painel é código aberto (supabase/supabase, packages/common/gotrue.ts, lido em 09/11/2026):
//! guarda o login no localStorage de supabase.com com a chave "supabase.dashboard.auth.token",
//! e usa o access_token dessa sessão para chamar a mesma API (api.supabase.com) que o app usa.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tauri::{Url, WebviewWindow};

use crate::error::AppResult;

const READ_JS: &str =
    r#"(function(){try{return window.localStorage.getItem("supabase.dashboard.auth.token")}catch(e){return null}})()"#;

/// A sessão vale se ainda tiver pelo menos 1 minuto pela frente.
const FRESH_MARGIN_SECS: i64 = 60;

#[derive(Debug, Clone, PartialEq)]
pub struct BrowserSession {
    pub access_token: String,
    /// Segundos desde 1970 em que o access_token vence.
    pub expires_at: Option<i64>,
}

impl BrowserSession {
    pub fn is_fresh(&self, now_secs: i64) -> bool {
        self.expires_at.is_none_or(|at| at > now_secs + FRESH_MARGIN_SECS)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reading {
    /// A janela está fora do supabase.com (no GitHub durante o login, por exemplo).
    Elsewhere,
    LoggedOut,
    LoggedIn(BrowserSession),
}

#[derive(Deserialize)]
struct StoredSession {
    access_token: Option<String>,
    expires_at: Option<i64>,
    /// Formato antigo do auth-js: a sessão vinha dentro de "currentSession".
    #[serde(rename = "currentSession")]
    current_session: Option<Box<StoredSession>>,
}

/// `eval_result` é o valor de localStorage já convertido em JSON pelo WebView2:
/// uma string JSON com a sessão dentro, ou `null`.
pub fn parse(eval_result: &str) -> Option<BrowserSession> {
    let raw = match serde_json::from_str::<Option<String>>(eval_result) {
        Ok(stored) => stored?,
        Err(_) => eval_result.to_string(),
    };
    let stored: StoredSession = serde_json::from_str(&raw).ok()?;
    let pick = |s: &StoredSession| {
        s.access_token
            .clone()
            .filter(|t| !t.is_empty())
            .map(|access_token| BrowserSession { access_token, expires_at: s.expires_at })
    };
    pick(&stored).or_else(|| stored.current_session.as_deref().and_then(pick))
}

pub fn is_supabase(url: &Url) -> bool {
    url.host_str().is_some_and(|h| h == "supabase.com" || h.ends_with(".supabase.com"))
}

/// Páginas onde não há sessão para esperar: o dono ainda precisa entrar.
pub fn is_auth_page(url: &Url) -> bool {
    is_supabase(url)
        && ["/sign-in", "/sign-up", "/forgot-password", "/sign-in-mfa"]
            .iter()
            .any(|p| url.path().contains(p))
}

pub async fn read(window: &WebviewWindow) -> AppResult<Reading> {
    let on_supabase = window.url().is_ok_and(|u| is_supabase(&u));
    if !on_supabase {
        return Ok(Reading::Elsewhere);
    }
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx = Mutex::new(Some(tx));
    window.eval_with_callback(READ_JS, move |result| {
        if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
            let _ = tx.send(result);
        }
    })?;
    Ok(match tokio::time::timeout(Duration::from_secs(5), rx).await {
        Ok(Ok(result)) => parse(&result).map_or(Reading::LoggedOut, Reading::LoggedIn),
        // A página não respondeu (ainda carregando): trata como fora do ar, não como deslogada.
        _ => Reading::Elsewhere,
    })
}

/// Espera o painel entregar uma sessão válida (ao abrir, ele renova o login sozinho).
/// Devolve `None` se a página parar no login ou se o tempo acabar.
pub async fn wait_fresh(window: &WebviewWindow, max: Duration) -> AppResult<Option<BrowserSession>> {
    let deadline = Instant::now() + max;
    loop {
        let now = chrono::Utc::now().timestamp();
        match read(window).await? {
            Reading::LoggedIn(s) if s.is_fresh(now) => return Ok(Some(s)),
            Reading::LoggedOut if window.url().is_ok_and(|u| is_auth_page(&u)) => return Ok(None),
            _ => {}
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        tokio::time::sleep(Duration::from_millis(700)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: &str = r#"{"access_token":"eyJ.a.b","token_type":"bearer","expires_in":3600,"expires_at":1800000000,"refresh_token":"r"}"#;

    #[test]
    fn parses_value_as_returned_by_webview() {
        let eval = serde_json::to_string(SESSION).unwrap();
        assert_eq!(
            parse(&eval),
            Some(BrowserSession { access_token: "eyJ.a.b".into(), expires_at: Some(1_800_000_000) })
        );
    }

    #[test]
    fn accepts_raw_value_and_old_format() {
        assert!(parse(SESSION).is_some());
        let old = serde_json::to_string(r#"{"currentSession":{"access_token":"velho","expires_at":5},"expiresAt":5}"#).unwrap();
        assert_eq!(parse(&old).unwrap().access_token, "velho");
    }

    #[test]
    fn no_session_means_none() {
        assert_eq!(parse("null"), None);
        assert_eq!(parse(&serde_json::to_string("lixo").unwrap()), None);
        assert_eq!(parse(&serde_json::to_string(r#"{"access_token":""}"#).unwrap()), None);
    }

    #[test]
    fn freshness_uses_one_minute_margin() {
        let s = BrowserSession { access_token: "t".into(), expires_at: Some(1_000) };
        assert!(s.is_fresh(900));
        assert!(!s.is_fresh(950));
    }

    #[test]
    fn recognizes_supabase_login_pages() {
        let u = |s: &str| Url::parse(s).unwrap();
        assert!(is_auth_page(&u("https://supabase.com/dashboard/sign-in?returnTo=%2Fprojects")));
        assert!(!is_auth_page(&u("https://supabase.com/dashboard/projects")));
        assert!(!is_auth_page(&u("https://github.com/login")));
        assert!(is_supabase(&u("https://supabase.com/dashboard")));
        assert!(!is_supabase(&u("https://supabase.com.evil.example/")));
    }
}
