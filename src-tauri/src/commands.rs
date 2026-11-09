//! Comandos que a tela chama. Todos são async: os que mexem em janela travariam o app no Windows
//! se rodassem na linha principal, e os outros não ganham nada sendo síncronos.

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use tauri::{AppHandle, State};

use crate::bundle::{self, ProjectInfo, ProjectSecrets};
use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{
    Account, AccountInput, AccountView, Overview, ProjectView, SessionStatus, TokenStatus, LOGIN_METHODS,
};
use crate::supabase::{self, ApiError};
use crate::sync::{self, now_text};
use crate::vault::{self, Secret};
use crate::{browser, slots, AppState};

/// Cor padrão quando a tela não manda uma válida.
const DEFAULT_COLOR: &str = "#62c9a5";

fn parse_time(text: Option<&str>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text?).ok().map(|t| t.with_timezone(&Utc))
}

fn build_view(app: &AppHandle, conn: &Connection, account: Account) -> AppResult<AccountView> {
    let organizations = db::list_organizations(conn, &account.id)?;
    let now = Utc::now();
    let mut slots_used = 0;
    let mut projects = Vec::new();
    for project in db::list_projects(conn, &account.id)? {
        let org = organizations.iter().find(|o| o.id == project.organization_id);
        let plan = org.and_then(|o| o.plan.clone());
        let counts = slots::counts_toward_limit(&project.status, plan.as_deref());
        if counts {
            slots_used += 1;
        }
        let pause = slots::pause_state(
            &project.status,
            plan.as_deref(),
            project.usage_checked,
            parse_time(project.last_activity_at.as_deref()),
            now,
        );
        projects.push(ProjectView {
            api_url: supabase::project_api_url(&project.project_ref),
            organization_name: org.map(|o| o.name.clone()),
            plan,
            counts_toward_limit: counts,
            pause,
            project,
        });
    }
    Ok(AccountView {
        has_password: vault::has(&account.id, Secret::Password),
        has_token: vault::has(&account.id, Secret::Token),
        browser_open: browser::is_open(app, &account.id),
        organizations,
        projects,
        slot_limit: slots::FREE_PROJECT_LIMIT,
        slots_used,
        slots_free: slots::slots_free(slots_used),
        account,
    })
}

fn view_by_id(app: &AppHandle, state: &AppState, id: &str) -> AppResult<AccountView> {
    let conn = state.conn()?;
    let account = db::get_account(&conn, id)?.ok_or(AppError::AccountNotFound)?;
    build_view(app, &conn, account)
}

fn overview(app: &AppHandle, state: &AppState) -> AppResult<Overview> {
    let conn = state.conn()?;
    let accounts = db::list_accounts(&conn)?
        .into_iter()
        .map(|a| build_view(app, &conn, a))
        .collect::<AppResult<Vec<_>>>()?;
    let candidates: Vec<_> = accounts
        .iter()
        .map(|a| slots::SlotCandidate {
            id: &a.account.id,
            token_ok: a.account.can_sync(),
            slots_free: a.slots_free,
            project_count: a.projects.len(),
        })
        .collect();
    let suggested_account_id = slots::suggest_account(&candidates).map(str::to_string);
    let total_free_slots = accounts
        .iter()
        .filter(|a| a.account.can_sync() && a.account.last_sync_at.is_some())
        .map(|a| a.slots_free)
        .sum();
    Ok(Overview { accounts, total_free_slots, suggested_account_id })
}

fn clean_color(color: &str) -> Option<String> {
    let color = color.trim().to_lowercase();
    let is_hex = color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    is_hex.then_some(color)
}

/// "Conta 1", "Conta 2"…: o primeiro número que ainda não está em uso.
fn next_alias(existing: &[String]) -> String {
    (1..)
        .map(|n| format!("Conta {n}"))
        .find(|alias| !existing.iter().any(|e| e.eq_ignore_ascii_case(alias)))
        .expect("sempre há um número livre")
}

struct CleanInput {
    alias: String,
    color: String,
    email: String,
    login_method: String,
    notes: String,
}

fn validate(input: &AccountInput) -> AppResult<CleanInput> {
    let alias = input.alias.trim().to_string();
    if alias.is_empty() || alias.chars().count() > 60 {
        return Err(AppError::Invalid("o apelido precisa ter de 1 a 60 letras".into()));
    }
    let color = clean_color(&input.color).ok_or_else(|| AppError::Invalid("cor inválida (use o formato #3ecf8e)".into()))?;
    if !LOGIN_METHODS.contains(&input.login_method.as_str()) {
        return Err(AppError::Invalid("forma de login inválida".into()));
    }
    Ok(CleanInput {
        alias,
        color,
        email: input.email.trim().to_string(),
        login_method: input.login_method.clone(),
        notes: input.notes.clone(),
    })
}

/// Grava ou apaga senha e token conforme o formulário e acerta o estado do token.
fn apply_secrets(conn: &Connection, id: &str, input: &AccountInput) -> AppResult<()> {
    if let Some(password) = &input.password {
        vault::set(id, Secret::Password, password)?;
    }
    if let Some(token) = &input.token {
        vault::set(id, Secret::Token, token)?;
        let status = if token.trim().is_empty() { TokenStatus::Missing } else { TokenStatus::Unchecked };
        db::set_token_status(conn, id, status)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn list_accounts(app: AppHandle, state: State<'_, AppState>) -> AppResult<Overview> {
    overview(&app, &state)
}

/// "Nova conta": cria a conta e já abre o navegador dela no login do Supabase.
/// Nada é pedido antes; o e-mail e os projetos vêm do login.
#[tauri::command]
pub async fn start_account(app: AppHandle, state: State<'_, AppState>, color: String) -> AppResult<AccountView> {
    let account = {
        let conn = state.conn()?;
        let account = Account {
            id: uuid::Uuid::new_v4().to_string(),
            alias: next_alias(&db::aliases(&conn)?),
            color: clean_color(&color).unwrap_or_else(|| DEFAULT_COLOR.to_string()),
            email: String::new(),
            login_method: "email".into(),
            notes: String::new(),
            username: None,
            session_status: SessionStatus::None,
            token_status: TokenStatus::Missing,
            last_sync_at: None,
            last_sync_error: None,
            created_at: now_text(),
        };
        db::insert_account(&conn, &account)?;
        account
    };
    browser::open(&app, &account, browser::target_url("signin", None)?)?;
    sync::watch(&app, &account.id);
    view_by_id(&app, &state, &account.id)
}

#[tauri::command]
pub async fn update_account(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: AccountInput,
) -> AppResult<AccountView> {
    let clean = validate(&input)?;
    {
        let conn = state.conn()?;
        let found = db::update_account_details(
            &conn,
            &id,
            &clean.alias,
            &clean.color,
            &clean.email,
            &clean.login_method,
            &clean.notes,
        )?;
        if !found {
            return Err(AppError::AccountNotFound);
        }
        apply_secrets(&conn, &id, &input)?;
    }
    if let Some(window) = browser::window(&app, &id) {
        let _ = window.set_title(&format!("{} · Ultrabase", clean.alias));
    }
    view_by_id(&app, &state, &id)
}

#[tauri::command]
pub async fn delete_account(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    browser::close(&app, &id);
    let refs: Vec<String> = {
        let conn = state.conn()?;
        db::list_projects(&conn, &id)?.into_iter().map(|p| p.project_ref).collect()
    };
    vault::delete_all(&id, &refs)?;
    {
        let conn = state.conn()?;
        db::delete_account(&conn, &id)?;
    }
    // Se o WebView2 ainda segurar os arquivos, a limpeza ao abrir o app termina o serviço.
    let _ = std::fs::remove_dir_all(browser::profiles_root(&app)?.join(&id));
    Ok(())
}

#[tauri::command]
pub async fn reveal_secret(id: String, kind: String) -> AppResult<Option<String>> {
    let secret = Secret::parse(&kind).ok_or_else(|| AppError::Invalid("segredo desconhecido".into()))?;
    vault::get(&id, secret)
}

#[tauri::command]
pub async fn sync_account(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<AccountView> {
    sync::sync_one(&app, &id).await?;
    view_by_id(&app, &state, &id)
}

/// Uma conta por vez, para não estourar o limite de pedidos. O erro de cada uma fica na própria conta.
#[tauri::command]
pub async fn sync_all(app: AppHandle, state: State<'_, AppState>) -> AppResult<Overview> {
    let ids = {
        let conn = state.conn()?;
        db::account_ids(&conn)?
    };
    for id in ids {
        let _ = sync::sync_one(&app, &id).await;
    }
    overview(&app, &state)
}

#[tauri::command]
pub async fn open_browser(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    target: String,
    project_ref: Option<String>,
) -> AppResult<()> {
    let account = {
        let conn = state.conn()?;
        db::get_account(&conn, &id)?.ok_or(AppError::AccountNotFound)?
    };
    // Conta ainda sem login abre direto na página de entrar.
    let target = if account.session_status == SessionStatus::Connected { target.as_str() } else { "signin" };
    let url = browser::target_url(target, project_ref.as_deref())?;
    browser::open(&app, &account, url)?;
    sync::watch(&app, &id);
    Ok(())
}

#[tauri::command]
pub async fn project_action(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    project_ref: String,
    action: String,
) -> AppResult<AccountView> {
    if !browser::is_valid_ref(&project_ref) {
        return Err(AppError::Invalid("projeto inválido".into()));
    }
    if action != "pause" && action != "restore" {
        return Err(AppError::Invalid(format!("ação desconhecida: {action}")));
    }
    let (client, source) = sync::client_for(&app, &id).await?;
    let (result, new_status) = if action == "pause" {
        (client.pause(&project_ref).await, "PAUSING")
    } else {
        (client.restore(&project_ref).await, "RESTORING")
    };
    if let Err(err) = result {
        return Err(sync::note_failure(&app, &id, source, err));
    }
    {
        let conn = state.conn()?;
        db::set_project_status(&conn, &id, &project_ref, new_status)?;
    }
    view_by_id(&app, &state, &id)
}

fn check_ref(project_ref: &str) -> AppResult<()> {
    if browser::is_valid_ref(project_ref) {
        Ok(())
    } else {
        Err(AppError::Invalid("projeto inválido".into()))
    }
}

/// Dado extra que não muda a conta (configurações, pooler): se falhar, segue sem ele.
/// Login recusado continua parando tudo.
fn optional<T>(result: Result<Option<T>, ApiError>) -> Result<Option<T>, ApiError> {
    match result {
        Err(ApiError::Unauthorized) => Err(ApiError::Unauthorized),
        Err(_) => Ok(None),
        ok => ok,
    }
}

/// Tudo do projeto (chaves, banco, conexões, token) e o texto pronto para a IA da IDE.
#[tauri::command]
pub async fn project_secrets(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    project_ref: String,
) -> AppResult<ProjectSecrets> {
    check_ref(&project_ref)?;
    let (account, project, org) = {
        let conn = state.conn()?;
        let account = db::get_account(&conn, &id)?.ok_or(AppError::AccountNotFound)?;
        let project = db::list_projects(&conn, &id)?
            .into_iter()
            .find(|p| p.project_ref == project_ref)
            .ok_or_else(|| AppError::Invalid("projeto não encontrado nesta conta; sincronize de novo".into()))?;
        let org = db::list_organizations(&conn, &id)?.into_iter().find(|o| o.id == project.organization_id);
        (account, project, org)
    };
    let (client, source) = sync::client_for(&app, &id).await?;
    let fetched = async {
        let keys = supabase::key_set(&client.api_keys(&project_ref).await?);
        let settings = optional(client.settings(&project_ref).await)?;
        let pooler = optional(client.pooler(&project_ref).await)?;
        Ok::<_, ApiError>((keys, settings, pooler))
    }
    .await;
    let (keys, settings, pooler) = fetched.map_err(|e| sync::note_failure(&app, &id, source, e))?;
    let info = ProjectInfo {
        project_ref: &project.project_ref,
        name: &project.name,
        region: &project.region,
        status: &project.status,
        organization_name: org.as_ref().map(|o| o.name.as_str()),
        plan: org.as_ref().and_then(|o| o.plan.as_deref()),
        account_alias: &account.alias,
        account_email: &account.email,
    };
    Ok(bundle::assemble(
        info,
        keys,
        settings,
        pooler,
        vault::get_db_password(&id, &project_ref)?,
        vault::get(&id, Secret::Token)?,
    ))
}

/// Guarda a senha do banco que o dono já tem (o Supabase não guarda e não devolve a senha).
#[tauri::command]
pub async fn set_db_password(id: String, project_ref: String, password: String) -> AppResult<()> {
    check_ref(&project_ref)?;
    vault::set_db_password(&id, &project_ref, &password)
}

/// 32 letras e números aleatórios: forte e sem caractere que precise de escape no endereço.
fn new_db_password() -> String {
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())[..32].to_string()
}

/// Troca a senha do banco no Supabase por uma nova e guarda no cofre. Conexões com a senha antiga param.
#[tauri::command]
pub async fn reset_db_password(app: AppHandle, id: String, project_ref: String) -> AppResult<()> {
    check_ref(&project_ref)?;
    let (client, source) = sync::client_for(&app, &id).await?;
    let password = new_db_password();
    // Guarda antes de trocar: se a troca der certo e o cofre falhasse depois, a senha nova se perderia.
    let previous = vault::get_db_password(&id, &project_ref)?;
    vault::set_db_password(&id, &project_ref, &password)?;
    if let Err(err) = client.reset_db_password(&project_ref, &password).await {
        vault::set_db_password(&id, &project_ref, previous.as_deref().unwrap_or(""))?;
        return Err(sync::note_failure(&app, &id, source, err));
    }
    Ok(())
}

/// Cria um token de acesso permanente na conta (para o MCP do Supabase na IDE) e guarda no cofre.
/// Também serve de reserva para ler a conta quando o login do navegador acabar.
#[tauri::command]
pub async fn create_ai_token(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<AccountView> {
    let alias = {
        let conn = state.conn()?;
        db::get_account(&conn, &id)?.ok_or(AppError::AccountNotFound)?.alias
    };
    let (client, source) = sync::client_for(&app, &id).await?;
    let token = client
        .create_access_token(&format!("Ultrabase ({alias})"))
        .await
        .map_err(|e| sync::note_failure(&app, &id, source, e))?;
    vault::set(&id, Secret::Token, &token)?;
    {
        let conn = state.conn()?;
        db::set_token_status(&conn, &id, TokenStatus::Ok)?;
    }
    view_by_id(&app, &state, &id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(alias: &str, color: &str, login_method: &str) -> AccountInput {
        AccountInput {
            alias: alias.into(),
            color: color.into(),
            email: " a@b.c ".into(),
            login_method: login_method.into(),
            notes: String::new(),
            password: None,
            token: None,
        }
    }

    #[test]
    fn validation_trims_and_checks_fields() {
        let ok = validate(&input("  pessoal ", "#3ECF8E", "github")).unwrap();
        assert_eq!((ok.alias.as_str(), ok.color.as_str(), ok.email.as_str()), ("pessoal", "#3ecf8e", "a@b.c"));
        assert!(validate(&input("   ", "#3ecf8e", "email")).is_err());
        assert!(validate(&input("x", "verde", "email")).is_err());
        assert!(validate(&input("x", "#3ecf8e", "google")).is_err());
    }

    #[test]
    fn generated_db_password_is_strong_and_url_safe() {
        let a = new_db_password();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_ne!(a, new_db_password());
    }

    #[test]
    fn next_alias_fills_first_free_number() {
        assert_eq!(next_alias(&[]), "Conta 1");
        let existing = vec!["Conta 1".to_string(), "conta 3".to_string(), "Pessoal".to_string()];
        assert_eq!(next_alias(&existing), "Conta 2");
    }
}
