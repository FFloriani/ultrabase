//! Cliente da API do Supabase (https://api.supabase.com), em duas famílias de rotas:
//! - pública (/v1): documentada em https://api.supabase.com/api/v1-json (lida em 09/11/2026); serve ao token pessoal.
//! - do painel (/platform): a que supabase.com/dashboard usa com o login do navegador. Formato conferido
//!   na captura do ReconLab de 09/11/2026 (supabase_20261109_133418). Com o login do navegador, a /v1
//!   devolve lista vazia de organizações e projetos (leitura real de 09/11/2026, conta com 1 projeto).

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use reqwest::{Method, StatusCode};
use serde::{de::DeserializeOwned, Deserialize};

const BASE_URL: &str = "https://api.supabase.com";

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("o Supabase recusou o token desta conta; crie um token novo")]
    Unauthorized,
    #[error("o Supabase pediu para esperar (limite de pedidos); tente de novo em 1 minuto")]
    RateLimited,
    #[error("o Supabase respondeu {status}: {body}")]
    Status { status: u16, body: String },
    #[error("sem conexão com o Supabase: {0}")]
    Network(String),
    #[error("resposta inesperada do Supabase: {0}")]
    Parse(String),
    #[error("isso só funciona com o login do navegador da conta; abra o navegador e entre no Supabase")]
    NeedsBrowserLogin,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiProfile {
    pub primary_email: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformPlan {
    /// "free", "pro", "team"…
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformOrg {
    /// Vem como número (ex.: 12345678); a /v1 usa texto.
    pub id: serde_json::Value,
    pub slug: String,
    pub name: String,
    /// Ex.: {"id": "free", "name": "Free"}.
    pub plan: Option<PlatformPlan>,
}

impl PlatformOrg {
    pub fn id_text(&self) -> String {
        match &self.id {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }
}

/// Família de rotas: decide pela origem do acesso, nunca tentando uma e caindo na outra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Routes {
    /// /platform: login do navegador.
    Dashboard,
    /// /v1: token pessoal.
    Public,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformProjectsResponse {
    pub projects: Vec<PlatformProject>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformProject {
    #[serde(rename = "ref")]
    pub project_ref: String,
    pub name: String,
    #[serde(default)]
    pub region: String,
    pub status: String,
    #[serde(alias = "created_at")]
    pub inserted_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiOrganization {
    pub id: String,
    pub slug: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiOrganizationDetail {
    pub plan: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiProject {
    /// A resposta traz `id` e `ref` com o mesmo valor; ler os dois no mesmo campo dá erro de campo repetido.
    #[serde(rename = "ref")]
    pub project_ref: String,
    pub name: String,
    #[serde(default)]
    pub region: String,
    pub organization_id: String,
    pub status: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKey {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub key_type: Option<String>,
    pub api_key: Option<String>,
}

/// GET /platform/projects/{ref}/settings (só no painel).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProjectSettings {
    pub db_host: Option<String>,
    pub db_name: Option<String>,
    pub db_user: Option<String>,
    pub db_port: Option<u16>,
    /// "ipv6" no plano grátis: a conexão direta só funciona em rede com IPv6.
    pub db_ip_addr_config: Option<String>,
    pub jwt_secret: Option<String>,
}

/// Pooler (Supavisor): /platform/projects/{ref}/config/supavisor ou /v1/projects/{ref}/config/database/pooler,
/// mesmo formato nas duas. A senha nunca vem: o Supabase não guarda a senha do banco.
#[derive(Debug, Clone, Deserialize)]
pub struct PoolerConfig {
    pub database_type: Option<String>,
    pub db_user: String,
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
}

#[derive(Deserialize)]
struct NewAccessToken {
    token: String,
}

/// Todas as chaves do projeto (a API só manda a secreta nova completa com reveal=true).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectKeySet {
    pub anon: Option<String>,
    pub publishable: Option<String>,
    pub service_role: Option<String>,
    pub secret: Option<String>,
}

pub struct Client {
    http: reqwest::Client,
    token: String,
    routes: Routes,
}

impl Client {
    pub fn new(http: reqwest::Client, token: String, routes: Routes) -> Self {
        Self { http, token, routes }
    }

    pub fn routes(&self) -> Routes {
        self.routes
    }

    async fn send(&self, method: Method, path: &str) -> Result<String, ApiError> {
        self.send_body(method, path, None).await
    }

    async fn send_body(&self, method: Method, path: &str, body: Option<serde_json::Value>) -> Result<String, ApiError> {
        let mut request = self.http.request(method, format!("{BASE_URL}{path}")).bearer_auth(&self.token);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let resp = request.send().await.map_err(|e| ApiError::Network(e.to_string()))?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| ApiError::Network(e.to_string()))?;
        match status {
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            StatusCode::TOO_MANY_REQUESTS => Err(ApiError::RateLimited),
            s if !s.is_success() => Err(ApiError::Status {
                status: s.as_u16(),
                body: body.chars().take(300).collect(),
            }),
            _ => Ok(body),
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let body = self.send(Method::GET, path).await?;
        serde_json::from_str(&body).map_err(|e| ApiError::Parse(e.to_string()))
    }

    pub async fn profile(&self) -> Result<ApiProfile, ApiError> {
        self.get("/v1/profile").await
    }

    pub async fn platform_organizations(&self) -> Result<Vec<PlatformOrg>, ApiError> {
        self.get("/platform/organizations").await
    }

    pub async fn platform_organization_projects(&self, slug: &str) -> Result<Vec<PlatformProject>, ApiError> {
        let resp: PlatformProjectsResponse = self
            .get(&format!(
                "/platform/organizations/{slug}/projects?limit=96&offset=0&sort=name_asc"
            ))
            .await?;
        Ok(resp.projects)
    }

    pub async fn organizations(&self) -> Result<Vec<ApiOrganization>, ApiError> {
        self.get("/v1/organizations").await
    }

    pub async fn organization_plan(&self, slug: &str) -> Result<Option<String>, ApiError> {
        let detail: ApiOrganizationDetail = self.get(&format!("/v1/organizations/{slug}")).await?;
        Ok(detail.plan)
    }

    pub async fn projects(&self) -> Result<Vec<ApiProject>, ApiError> {
        self.get("/v1/projects").await
    }

    /// Último pedido contado pela API nos últimos 7 dias; `None` = nenhum pedido.
    /// O painel chama a mesma leitura em /platform/projects/{ref}/analytics/endpoints/usage.api-counts.
    pub async fn last_activity(&self, project_ref: &str) -> Result<Option<DateTime<Utc>>, ApiError> {
        let prefix = match self.routes {
            Routes::Dashboard => "/platform",
            Routes::Public => "/v1",
        };
        let value: serde_json::Value = self
            .get(&format!(
                "{prefix}/projects/{project_ref}/analytics/endpoints/usage.api-counts?interval=7day"
            ))
            .await?;
        last_activity_from_usage(&value)
    }

    pub async fn api_keys(&self, project_ref: &str) -> Result<Vec<ApiKey>, ApiError> {
        self.get(&format!("/v1/projects/{project_ref}/api-keys?reveal=true")).await
    }

    /// Configurações do banco e o JWT secret. Só existe nas rotas do painel; com token pessoal volta `None`.
    pub async fn settings(&self, project_ref: &str) -> Result<Option<ProjectSettings>, ApiError> {
        match self.routes {
            Routes::Dashboard => self.get(&format!("/platform/projects/{project_ref}/settings")).await.map(Some),
            Routes::Public => Ok(None),
        }
    }

    /// O pooler do banco principal.
    pub async fn pooler(&self, project_ref: &str) -> Result<Option<PoolerConfig>, ApiError> {
        let path = match self.routes {
            Routes::Dashboard => format!("/platform/projects/{project_ref}/config/supavisor"),
            Routes::Public => format!("/v1/projects/{project_ref}/config/database/pooler"),
        };
        let list: Vec<PoolerConfig> = self.get(&path).await?;
        let primary = list.iter().position(|p| p.database_type.as_deref() == Some("PRIMARY")).unwrap_or(0);
        Ok(list.into_iter().nth(primary))
    }

    /// Cria um token de acesso permanente na conta (o mesmo de Account → Access Tokens).
    /// Rota do painel: POST /platform/profile/access-tokens {name}.
    pub async fn create_access_token(&self, name: &str) -> Result<String, ApiError> {
        if self.routes != Routes::Dashboard {
            return Err(ApiError::NeedsBrowserLogin);
        }
        let body = self
            .send_body(Method::POST, "/platform/profile/access-tokens", Some(serde_json::json!({ "name": name })))
            .await?;
        let created: NewAccessToken = serde_json::from_str(&body).map_err(|e| ApiError::Parse(e.to_string()))?;
        Ok(created.token)
    }

    /// Troca a senha do banco. Rota do painel: PATCH /platform/projects/{ref}/db-password {password}.
    pub async fn reset_db_password(&self, project_ref: &str, password: &str) -> Result<(), ApiError> {
        if self.routes != Routes::Dashboard {
            return Err(ApiError::NeedsBrowserLogin);
        }
        self.send_body(
            Method::PATCH,
            &format!("/platform/projects/{project_ref}/db-password"),
            Some(serde_json::json!({ "password": password })),
        )
        .await
        .map(|_| ())
    }

    pub async fn pause(&self, project_ref: &str) -> Result<(), ApiError> {
        self.send(Method::POST, &format!("/v1/projects/{project_ref}/pause")).await.map(|_| ())
    }

    pub async fn restore(&self, project_ref: &str) -> Result<(), ApiError> {
        self.send(Method::POST, &format!("/v1/projects/{project_ref}/restore")).await.map(|_| ())
    }
}

pub fn project_api_url(project_ref: &str) -> String {
    format!("https://{project_ref}.supabase.co")
}

/// Separa as chaves por tipo: antigas (anon e service_role, em JWT) e novas (publishable e secret).
pub fn key_set(keys: &[ApiKey]) -> ProjectKeySet {
    let find = |key_type: &str, name: Option<&str>| {
        keys.iter()
            .find(|k| k.key_type.as_deref() == Some(key_type) && name.is_none_or(|n| k.name.as_deref() == Some(n)))
            .and_then(|k| k.api_key.clone())
            .filter(|v| !v.is_empty())
    };
    ProjectKeySet {
        anon: find("legacy", Some("anon")),
        service_role: find("legacy", Some("service_role")),
        publishable: find("publishable", None),
        secret: find("secret", None),
    }
}

/// A resposta traz `result`: uma lista de faixas de tempo com `timestamp` e contadores
/// (pedidos de REST, login, storage, realtime). Vale a faixa mais nova com algum pedido.
pub fn last_activity_from_usage(value: &serde_json::Value) -> Result<Option<DateTime<Utc>>, ApiError> {
    if let Some(err) = value.get("error").filter(|e| !e.is_null()) {
        return Err(ApiError::Parse(format!("uso indisponível: {err}")));
    }
    let rows = value
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| ApiError::Parse("uso sem a lista \"result\"".into()))?;
    let latest = rows
        .iter()
        .filter_map(|row| {
            let obj = row.as_object()?;
            let total: f64 = obj
                .iter()
                .filter(|(k, _)| k.as_str() != "timestamp")
                .filter_map(|(_, v)| v.as_f64())
                .sum();
            (total > 0.0).then(|| parse_timestamp(obj.get("timestamp")?)).flatten()
        })
        .max();
    Ok(latest)
}

/// O Supabase manda a hora como texto ISO ou como número (segundos, milissegundos ou microssegundos).
fn parse_timestamp(value: &serde_json::Value) -> Option<DateTime<Utc>> {
    if let Some(n) = value.as_i64() {
        let micros = match n.abs() {
            x if x >= 100_000_000_000_000 => n,
            x if x >= 100_000_000_000 => n * 1_000,
            _ => n * 1_000_000,
        };
        return Utc.timestamp_micros(micros).single();
    }
    let text = value.as_str()?;
    if let Ok(dt) = DateTime::parse_from_rfc3339(text) {
        return Some(dt.with_timezone(&Utc));
    }
    ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"]
        .iter()
        .find_map(|fmt| NaiveDateTime::parse_from_str(text, fmt).ok())
        .map(|naive| naive.and_utc())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_projects_list() {
        let body = r#"[{"id":"abcdefghijklmnopqrst","ref":"abcdefghijklmnopqrst","organization_id":"org1",
            "organization_slug":"org1","name":"loja","region":"sa-east-1","created_at":"2026-10-01T10:00:00Z",
            "status":"ACTIVE_HEALTHY","database":{"host":"db.abc.supabase.co","version":"17"}}]"#;
        let projects: Vec<ApiProject> = serde_json::from_str(body).unwrap();
        assert_eq!(projects[0].project_ref, "abcdefghijklmnopqrst");
        assert_eq!(projects[0].region, "sa-east-1");
        assert_eq!(projects[0].status, "ACTIVE_HEALTHY");
    }

    /// Formatos da captura real de 09/11/2026 (valores trocados).
    #[test]
    fn parses_dashboard_organizations_and_projects() {
        let orgs: Vec<PlatformOrg> = serde_json::from_value(json!([{
            "id": 12345678, "slug": "abcdefghijklmnopqrst", "name": "Minha org", "is_owner": true,
            "plan": {"id": "free", "name": "Free"}, "tier": "tier_free", "org_id": 12345678
        }]))
        .unwrap();
        assert_eq!(orgs[0].id_text(), "12345678");
        assert_eq!(orgs[0].plan.as_ref().and_then(|p| p.id.as_deref()), Some("free"));

        let page: PlatformProjectsResponse = serde_json::from_value(json!({
            "projects": [{
                "ref": "qwertyuiopasdfghjklz", "name": "loja", "cloud_provider": "AWS", "region": "us-east-1",
                "is_branch": false, "status": "ACTIVE_HEALTHY", "inserted_at": "2026-11-09T15:29:22.037664",
                "databases": [{"identifier": "qwertyuiopasdfghjklz", "type": "PRIMARY"}], "integration_source": null
            }],
            "pagination": {"count": 1, "limit": 96, "offset": 0}
        }))
        .unwrap();
        let p = &page.projects[0];
        assert_eq!((p.project_ref.as_str(), p.region.as_str(), p.status.as_str()), ("qwertyuiopasdfghjklz", "us-east-1", "ACTIVE_HEALTHY"));
        assert_eq!(p.inserted_at.as_deref(), Some("2026-11-09T15:29:22.037664"));
    }

    #[test]
    fn usage_picks_latest_bucket_with_requests() {
        let usage = json!({"result": [
            {"timestamp": "2026-11-05T00:00:00Z", "total_rest_requests": 4, "total_auth_requests": 0},
            {"timestamp": "2026-11-07T00:00:00Z", "total_rest_requests": 0, "total_auth_requests": 2},
            {"timestamp": "2026-11-08T00:00:00Z", "total_rest_requests": 0, "total_auth_requests": 0}
        ], "error": null});
        let at = last_activity_from_usage(&usage).unwrap().unwrap();
        assert_eq!(at.to_rfc3339(), "2026-11-07T00:00:00+00:00");
    }

    #[test]
    fn usage_accepts_numeric_microsecond_timestamps() {
        let usage = json!({"result": [{"timestamp": 1_762_473_600_000_000_i64, "total_rest_requests": 1}]});
        let at = last_activity_from_usage(&usage).unwrap().unwrap();
        assert_eq!(at.timestamp(), 1_762_473_600);
    }

    #[test]
    fn usage_without_requests_means_no_activity() {
        let usage = json!({"result": [{"timestamp": "2026-11-08 00:00:00", "total_rest_requests": 0}]});
        assert_eq!(last_activity_from_usage(&usage).unwrap(), None);
    }

    #[test]
    fn usage_error_is_reported() {
        let usage = json!({"result": [], "error": "boom"});
        assert!(last_activity_from_usage(&usage).is_err());
    }

    #[test]
    fn key_set_separates_every_key_type() {
        let keys: Vec<ApiKey> = serde_json::from_value(json!([
            {"name": "anon", "type": "legacy", "api_key": "anon-jwt"},
            {"name": "service_role", "type": "legacy", "api_key": "service-jwt"},
            {"name": "default", "type": "publishable", "api_key": "sb_publishable_x"},
            {"name": "default", "type": "secret", "api_key": "sb_secret_x"}
        ]))
        .unwrap();
        assert_eq!(
            key_set(&keys),
            ProjectKeySet {
                anon: Some("anon-jwt".into()),
                publishable: Some("sb_publishable_x".into()),
                service_role: Some("service-jwt".into()),
                secret: Some("sb_secret_x".into()),
            }
        );
        assert_eq!(key_set(&[]), ProjectKeySet::default());
    }

    /// Formatos da captura real de 09/11/2026 (valores trocados).
    #[test]
    fn parses_settings_and_pooler() {
        let s: ProjectSettings = serde_json::from_value(json!({
            "name": "loja", "ref": "abc", "db_host": "db.abc.supabase.co", "db_name": "postgres", "db_user": "postgres",
            "db_port": 5432, "db_ip_addr_config": "ipv6", "jwt_secret": "segredo",
            "app_config": {"db_schema": "public", "endpoint": "abc.supabase.co"},
            "service_api_keys": [{"api_key": "x", "name": "anon key", "tags": "anon"}]
        }))
        .unwrap();
        assert_eq!((s.db_port, s.db_ip_addr_config.as_deref(), s.jwt_secret.as_deref()), (Some(5432), Some("ipv6"), Some("segredo")));
        let pooler: Vec<PoolerConfig> = serde_json::from_value(json!([{
            "identifier": "abc", "database_type": "PRIMARY", "is_using_scram_auth": true, "db_user": "postgres.abc",
            "db_host": "aws-0-us-east-1.pooler.supabase.com", "db_port": 6543, "db_name": "postgres",
            "connection_string": "postgresql://postgres.abc:[YOUR-PASSWORD]@aws-0-us-east-1.pooler.supabase.com:6543/postgres",
            "default_pool_size": null, "max_client_conn": null, "pool_mode": "transaction"
        }]))
        .unwrap();
        assert_eq!(pooler[0].db_host, "aws-0-us-east-1.pooler.supabase.com");
    }
}
