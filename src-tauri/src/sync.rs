//! Conectar e ler uma conta: de onde vem o acesso, a leitura no Supabase e o vigia do login.
//! O acesso vem primeiro do login guardado no navegador da conta; o token pessoal é só reserva.

use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::db::{self, SyncProfile};
use crate::error::{AppError, AppResult};
use crate::models::{Account, Organization, Project, SessionStatus, TokenStatus};
use crate::session::{self, Reading};
use crate::supabase::{ApiError, Client, Routes};
use crate::vault::{self, Secret};
use crate::{browser, AppState};

pub const SESSION_EVENT: &str = "session-changed";
pub const SYNC_EVENT: &str = "account-synced";

const DISCONNECTED: &str =
    "conta desconectada: clique em \"Abrir navegador\" e entre no Supabase nessa janela";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Session,
    Token,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionEvent {
    account_id: String,
    status: SessionStatus,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncEvent {
    account_id: String,
    error: Option<String>,
}

pub fn now_text() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn get_account(app: &AppHandle, id: &str) -> AppResult<Account> {
    let state = app.state::<AppState>();
    let conn = state.conn()?;
    db::get_account(&conn, id)?.ok_or(AppError::AccountNotFound)
}

/// Grava o estado do login e avisa a tela, só quando muda.
fn set_session(app: &AppHandle, id: &str, status: SessionStatus) -> AppResult<()> {
    {
        let state = app.state::<AppState>();
        let conn = state.conn()?;
        let before = db::get_account(&conn, id)?.map(|a| a.session_status);
        if before == Some(status) {
            return Ok(());
        }
        db::set_session_status(&conn, id, status)?;
    }
    let _ = app.emit(SESSION_EVENT, SessionEvent { account_id: id.to_string(), status });
    Ok(())
}

/// Único lugar que decide com que credencial o app fala com o Supabase por uma conta.
pub async fn credential(app: &AppHandle, account: &Account) -> AppResult<(String, Source)> {
    let (window, temporary) = match browser::window(app, &account.id) {
        Some(window) => (window, false),
        None => (browser::open_hidden(app, account)?, true),
    };
    // Janela nova precisa carregar o painel, que renova o login sozinho ao abrir.
    let wait = Duration::from_secs(if temporary { 25 } else { 12 });
    let session = session::wait_fresh(&window, wait).await;
    if temporary {
        browser::close_if_hidden(&window);
    }
    match session {
        Ok(Some(s)) => {
            set_session(app, &account.id, SessionStatus::Connected)?;
            return Ok((s.access_token, Source::Session));
        }
        Ok(None) if account.session_status == SessionStatus::Connected => {
            set_session(app, &account.id, SessionStatus::Expired)?;
        }
        _ => {}
    }
    match vault::get(&account.id, Secret::Token)? {
        Some(token) => Ok((token, Source::Token)),
        None => Err(AppError::Invalid(DISCONNECTED.into())),
    }
}

pub async fn client_for(app: &AppHandle, id: &str) -> AppResult<(Client, Source)> {
    let account = get_account(app, id)?;
    let (token, source) = match credential(app, &account).await {
        Ok(found) => found,
        Err(err) => {
            let state = app.state::<AppState>();
            db::save_sync_error(&*state.conn()?, id, &err.to_string())?;
            return Err(err);
        }
    };
    let http = app.state::<AppState>().http.clone();
    let routes = match source {
        Source::Session => Routes::Dashboard,
        Source::Token => Routes::Public,
    };
    Ok((Client::new(http, token, routes), source))
}

/// Único lugar que trata falha da API: credencial recusada marca o que precisa ser refeito, a
/// mensagem fica guardada na conta e a mesma mensagem volta para a tela.
pub fn note_failure(app: &AppHandle, id: &str, source: Source, err: ApiError) -> AppError {
    let unauthorized = matches!(err, ApiError::Unauthorized);
    let message = match source {
        Source::Session if unauthorized => {
            "o Supabase recusou o login desta conta; abra o navegador e entre de novo".to_string()
        }
        _ => err.to_string(),
    };
    let recorded = (|| -> AppResult<()> {
        if unauthorized {
            match source {
                Source::Session => set_session(app, id, SessionStatus::Expired)?,
                Source::Token => {
                    let state = app.state::<AppState>();
                    db::set_token_status(&*state.conn()?, id, TokenStatus::Invalid)?;
                }
            }
        }
        let state = app.state::<AppState>();
        db::save_sync_error(&*state.conn()?, id, &message)?;
        Ok(())
    })();
    match recorded {
        Ok(()) => AppError::Invalid(message),
        Err(db_err) => db_err,
    }
}

struct Snapshot {
    username: Option<String>,
    primary_email: Option<String>,
    organizations: Vec<Organization>,
    projects: Vec<Project>,
}

/// Projeto como veio da lista, antes de ler o uso.
struct Listed {
    project_ref: String,
    name: String,
    region: String,
    organization_id: String,
    status: String,
    created_at: Option<String>,
}

/// Rotas do painel (login do navegador).
async fn list_dashboard(client: &Client) -> Result<(Vec<Organization>, Vec<Listed>), ApiError> {
    let mut organizations = Vec::new();
    let mut listed = Vec::new();
    for org in client.platform_organizations().await? {
        let org_id = org.id_text();
        // Falha aqui interrompe a leitura de propósito: gravar a organização sem os projetos dela
        // mostraria vagas livres que não existem e apagaria a lista boa da leitura anterior.
        for p in client.platform_organization_projects(&org.slug).await? {
            listed.push(Listed {
                project_ref: p.project_ref,
                name: p.name,
                region: p.region,
                organization_id: org_id.clone(),
                status: p.status,
                created_at: p.inserted_at,
            });
        }
        organizations.push(Organization {
            id: org_id,
            slug: org.slug,
            name: org.name,
            plan: org.plan.and_then(|p| p.id),
        });
    }
    Ok((organizations, listed))
}

/// API pública /v1 (token pessoal).
async fn list_public(client: &Client) -> Result<(Vec<Organization>, Vec<Listed>), ApiError> {
    let mut organizations = Vec::new();
    for org in client.organizations().await? {
        let slug = org.slug.clone().unwrap_or_else(|| org.id.clone());
        // Sem o plano a conta ainda funciona: o desconhecido conta como grátis.
        let plan = match client.organization_plan(&slug).await {
            Ok(plan) => plan,
            Err(ApiError::Unauthorized) => return Err(ApiError::Unauthorized),
            Err(_) => None,
        };
        organizations.push(Organization { id: org.id, slug, name: org.name, plan });
    }
    let listed = client
        .projects()
        .await?
        .into_iter()
        .map(|p| Listed {
            project_ref: p.project_ref,
            name: p.name,
            region: p.region,
            organization_id: p.organization_id,
            status: p.status,
            created_at: p.created_at,
        })
        .collect();
    Ok((organizations, listed))
}

/// Junta o uso dos últimos 7 dias ao projeto. Pausado não tem uso para ler; nos outros, falha no
/// uso vira "sem estimativa", menos login recusado, que para tudo.
async fn with_usage(client: &Client, l: Listed) -> Result<Project, ApiError> {
    let (last_activity_at, usage_checked) = if l.status == "INACTIVE" {
        (None, false)
    } else {
        match client.last_activity(&l.project_ref).await {
            Ok(at) => (at.map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true)), true),
            Err(ApiError::Unauthorized) => return Err(ApiError::Unauthorized),
            Err(_) => (None, false),
        }
    };
    Ok(Project {
        project_ref: l.project_ref,
        name: l.name,
        region: l.region,
        organization_id: l.organization_id,
        status: l.status,
        created_at: l.created_at,
        last_activity_at,
        usage_checked,
    })
}

async fn fetch_snapshot(client: &Client) -> Result<Snapshot, ApiError> {
    let profile = client.profile().await?;
    let (organizations, listed) = match client.routes() {
        Routes::Dashboard => list_dashboard(client).await?,
        Routes::Public => list_public(client).await?,
    };
    let mut projects = Vec::with_capacity(listed.len());
    for l in listed {
        projects.push(with_usage(client, l).await?);
    }
    Ok(Snapshot { username: profile.username, primary_email: profile.primary_email, organizations, projects })
}

pub async fn sync_one(app: &AppHandle, id: &str) -> AppResult<()> {
    let (client, source) = client_for(app, id).await?;
    match fetch_snapshot(&client).await {
        Ok(snap) => {
            let state = app.state::<AppState>();
            let mut conn = state.conn()?;
            let profile = SyncProfile { username: snap.username.as_deref(), primary_email: snap.primary_email.as_deref() };
            db::save_sync(&mut conn, id, profile, &snap.organizations, &snap.projects, &now_text())?;
            if source == Source::Token {
                db::set_token_status(&conn, id, TokenStatus::Ok)?;
            }
            Ok(())
        }
        Err(err) => Err(note_failure(app, id, source, err)),
    }
}

/// Vigia o navegador visível da conta: quando o dono entra no Supabase, marca a conta como
/// conectada e lê tudo na hora; quando ele sai, marca como desconectada. Para sozinho quando a
/// janela fecha. Uma vigia por conta.
pub fn watch(app: &AppHandle, account_id: &str) {
    let started = app
        .state::<AppState>()
        .watching
        .lock()
        .map(|mut set| set.insert(account_id.to_string()))
        .unwrap_or(false);
    if !started {
        return;
    }
    let app = app.clone();
    let id = account_id.to_string();
    tauri::async_runtime::spawn(async move {
        // Login já tentado: se o Supabase recusou, só tenta de novo quando o login mudar
        // (senão o vigia repetiria a mesma leitura recusada a cada volta).
        let mut tried: Option<String> = None;
        loop {
            tokio::time::sleep(Duration::from_millis(2500)).await;
            let Some(window) = browser::window(&app, &id) else { break };
            if !window.is_visible().unwrap_or(false) {
                continue;
            }
            let Ok(account) = get_account(&app, &id) else { break };
            let reading = session::read(&window).await.unwrap_or(Reading::Elsewhere);
            let now = Utc::now().timestamp();
            match reading {
                Reading::LoggedIn(s)
                    if s.is_fresh(now)
                        && account.session_status != SessionStatus::Connected
                        && tried.as_deref() != Some(s.access_token.as_str()) =>
                {
                    tried = Some(s.access_token.clone());
                    let _ = set_session(&app, &id, SessionStatus::Connected);
                    let error = sync_one(&app, &id).await.err().map(|e| e.to_string());
                    let _ = app.emit(SYNC_EVENT, SyncEvent { account_id: id.clone(), error });
                }
                Reading::LoggedOut if account.session_status == SessionStatus::Connected => {
                    let _ = set_session(&app, &id, SessionStatus::Expired);
                }
                _ => {}
            }
        }
        if let Ok(mut set) = app.state::<AppState>().watching.lock() {
            set.remove(&id);
        }
    });
}
