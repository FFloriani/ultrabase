use serde::{Deserialize, Serialize};

pub const LOGIN_METHODS: [&str; 2] = ["email", "github"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenStatus {
    /// Nenhum token guardado no cofre.
    Missing,
    /// Token guardado, ainda não testado.
    Unchecked,
    Ok,
    /// O Supabase recusou o token (401).
    Invalid,
}

impl TokenStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TokenStatus::Missing => "missing",
            TokenStatus::Unchecked => "unchecked",
            TokenStatus::Ok => "ok",
            TokenStatus::Invalid => "invalid",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "unchecked" => TokenStatus::Unchecked,
            "ok" => TokenStatus::Ok,
            "invalid" => TokenStatus::Invalid,
            _ => TokenStatus::Missing,
        }
    }
}

/// Login do Supabase dentro do navegador da conta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Nunca entrou: navegador criado, esperando o login.
    None,
    Connected,
    /// Já esteve conectada, mas a sessão acabou (saiu ou venceu).
    Expired,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionStatus::None => "none",
            SessionStatus::Connected => "connected",
            SessionStatus::Expired => "expired",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "connected" => SessionStatus::Connected,
            "expired" => SessionStatus::Expired,
            _ => SessionStatus::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub alias: String,
    pub color: String,
    pub email: String,
    pub login_method: String,
    pub notes: String,
    /// Nome de usuário que o Supabase devolve no perfil.
    pub username: Option<String>,
    pub session_status: SessionStatus,
    /// Token pessoal: opcional, serve para sincronizar sem o navegador.
    pub token_status: TokenStatus,
    pub last_sync_at: Option<String>,
    pub last_sync_error: Option<String>,
    pub created_at: String,
}

impl Account {
    /// Dá para ler a conta: pelo login do navegador ou pelo token pessoal.
    pub fn can_sync(&self) -> bool {
        self.session_status == SessionStatus::Connected || self.token_status == TokenStatus::Ok
    }
}

/// Dados do formulário de conta. Em `password` e `token`: ausente = não mexe,
/// texto vazio = apaga do cofre, texto = grava no cofre.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
    pub alias: String,
    pub color: String,
    pub email: String,
    pub login_method: String,
    pub notes: String,
    pub password: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    pub id: String,
    pub slug: String,
    pub name: String,
    /// free, pro, team, enterprise. `None` quando o Supabase não respondeu o plano.
    pub plan: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(rename = "ref")]
    pub project_ref: String,
    pub name: String,
    pub region: String,
    pub organization_id: String,
    pub status: String,
    pub created_at: Option<String>,
    /// Último pedido que a API contou nos últimos 7 dias.
    pub last_activity_at: Option<String>,
    /// `true` quando o uso foi lido; sem isso não dá para estimar a pausa.
    pub usage_checked: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PauseState {
    Paused,
    /// Organização paga: não pausa por falta de uso.
    Paid,
    Unknown,
    Days { days: i64 },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    #[serde(flatten)]
    pub project: Project,
    pub organization_name: Option<String>,
    pub plan: Option<String>,
    pub counts_toward_limit: bool,
    pub pause: PauseState,
    pub api_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    #[serde(flatten)]
    pub account: Account,
    pub has_password: bool,
    pub has_token: bool,
    pub organizations: Vec<Organization>,
    pub projects: Vec<ProjectView>,
    pub slot_limit: u32,
    pub slots_used: u32,
    pub slots_free: u32,
    pub browser_open: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub accounts: Vec<AccountView>,
    pub total_free_slots: u32,
    pub suggested_account_id: Option<String>,
}
