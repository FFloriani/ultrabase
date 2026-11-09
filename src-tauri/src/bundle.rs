//! Tudo de um projeto num lugar só (chaves, banco, conexões, token) e o texto que vai para a IA
//! da IDE com um clique. Sem rede nem cofre aqui: só monta, para dar para testar sozinho.

use serde::Serialize;

use crate::supabase::{PoolerConfig, ProjectKeySet, ProjectSettings};

/// O mesmo texto que o painel do Supabase mostra no lugar da senha, que ele não guarda.
pub const PASSWORD_PLACEHOLDER: &str = "[YOUR-PASSWORD]";

/// Porta do pooler em modo sessão (o modo transação vem na configuração, normalmente 6543).
const SESSION_POOLER_PORT: u16 = 5432;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSecrets {
    pub project_ref: String,
    pub name: String,
    pub region: String,
    pub status: String,
    pub organization_name: Option<String>,
    pub plan: Option<String>,
    pub api_url: String,
    pub dashboard_url: String,
    pub anon_key: Option<String>,
    pub publishable_key: Option<String>,
    pub service_role_key: Option<String>,
    pub secret_key: Option<String>,
    pub jwt_secret: Option<String>,
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub db_user: String,
    /// Conexão direta só por IPv6 (padrão do plano grátis).
    pub ipv6_only: bool,
    pub db_password: Option<String>,
    pub direct_url: String,
    pub pooler_transaction_url: Option<String>,
    pub pooler_session_url: Option<String>,
    /// Token de acesso da conta (para o servidor MCP do Supabase na IDE).
    pub access_token: Option<String>,
    /// O texto completo para colar na IA.
    pub bundle: String,
}

pub struct ProjectInfo<'a> {
    pub project_ref: &'a str,
    pub name: &'a str,
    pub region: &'a str,
    pub status: &'a str,
    pub organization_name: Option<&'a str>,
    pub plan: Option<&'a str>,
    pub account_alias: &'a str,
    pub account_email: &'a str,
}

/// Senha dentro de um endereço postgresql://: tudo que não é letra, número ou -._~ vira %XX.
fn encode_userinfo(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn pg_url(user: &str, password: Option<&str>, host: &str, port: u16, db: &str) -> String {
    let password = password.map(encode_userinfo).unwrap_or_else(|| PASSWORD_PLACEHOLDER.to_string());
    format!("postgresql://{user}:{password}@{host}:{port}/{db}")
}

pub fn assemble(
    info: ProjectInfo,
    keys: ProjectKeySet,
    settings: Option<ProjectSettings>,
    pooler: Option<PoolerConfig>,
    db_password: Option<String>,
    access_token: Option<String>,
) -> ProjectSecrets {
    let r = info.project_ref;
    let settings = settings.unwrap_or_default();
    let db_host = settings.db_host.clone().unwrap_or_else(|| format!("db.{r}.supabase.co"));
    let db_port = settings.db_port.unwrap_or(5432);
    let db_name = settings.db_name.clone().unwrap_or_else(|| "postgres".into());
    let db_user = settings.db_user.clone().unwrap_or_else(|| "postgres".into());
    let pw = db_password.as_deref();

    let direct_url = pg_url(&db_user, pw, &db_host, db_port, &db_name);
    let pooler_transaction_url = pooler.as_ref().map(|p| pg_url(&p.db_user, pw, &p.db_host, p.db_port, &p.db_name));
    let pooler_session_url =
        pooler.as_ref().map(|p| pg_url(&p.db_user, pw, &p.db_host, SESSION_POOLER_PORT, &p.db_name));

    let mut s = ProjectSecrets {
        project_ref: r.to_string(),
        name: info.name.to_string(),
        region: info.region.to_string(),
        status: info.status.to_string(),
        organization_name: info.organization_name.map(str::to_string),
        plan: info.plan.map(str::to_string),
        api_url: format!("https://{r}.supabase.co"),
        dashboard_url: format!("https://supabase.com/dashboard/project/{r}"),
        anon_key: keys.anon,
        publishable_key: keys.publishable,
        service_role_key: keys.service_role,
        secret_key: keys.secret,
        jwt_secret: settings.jwt_secret.clone(),
        db_host,
        db_port,
        db_name,
        db_user,
        ipv6_only: settings.db_ip_addr_config.as_deref() == Some("ipv6"),
        db_password,
        direct_url,
        pooler_transaction_url,
        pooler_session_url,
        access_token,
        bundle: String::new(),
    };
    s.bundle = render(&s, &info);
    s
}

fn line(out: &mut String, key: &str, value: Option<&str>) {
    if let Some(v) = value {
        out.push_str(&format!("{key}={v}\n"));
    }
}

fn render(s: &ProjectSecrets, info: &ProjectInfo) -> String {
    let mut env = String::new();
    env.push_str("# Cliente: pode ir para o navegador (o acesso fica protegido pelas regras de RLS)\n");
    line(&mut env, "SUPABASE_URL", Some(&s.api_url));
    line(&mut env, "SUPABASE_ANON_KEY", s.anon_key.as_deref());
    line(&mut env, "SUPABASE_PUBLISHABLE_KEY", s.publishable_key.as_deref());
    line(&mut env, "NEXT_PUBLIC_SUPABASE_URL", Some(&s.api_url));
    line(&mut env, "NEXT_PUBLIC_SUPABASE_ANON_KEY", s.anon_key.as_deref());
    line(&mut env, "VITE_SUPABASE_URL", Some(&s.api_url));
    line(&mut env, "VITE_SUPABASE_ANON_KEY", s.anon_key.as_deref());
    env.push_str("\n# Servidor: SECRETO, passa por cima de todas as regras de RLS. Nunca no navegador nem no git.\n");
    line(&mut env, "SUPABASE_SERVICE_ROLE_KEY", s.service_role_key.as_deref());
    line(&mut env, "SUPABASE_SECRET_KEY", s.secret_key.as_deref());
    line(&mut env, "SUPABASE_JWT_SECRET", s.jwt_secret.as_deref());
    env.push_str("\n# Banco Postgres\n");
    line(&mut env, "SUPABASE_DB_PASSWORD", Some(s.db_password.as_deref().unwrap_or(PASSWORD_PLACEHOLDER)));
    // Pooler em modo transação para o app; modo sessão para migrações (funciona em rede só IPv4).
    line(&mut env, "DATABASE_URL", Some(s.pooler_transaction_url.as_deref().unwrap_or(&s.direct_url)));
    line(&mut env, "DIRECT_URL", Some(s.pooler_session_url.as_deref().unwrap_or(&s.direct_url)));
    line(&mut env, "SUPABASE_DB_DIRECT_URL", Some(&s.direct_url));
    env.push_str("\n# Gerenciamento do projeto pela API do Supabase (o token vale para a conta inteira)\n");
    line(&mut env, "SUPABASE_PROJECT_REF", Some(&s.project_ref));
    line(&mut env, "SUPABASE_ACCESS_TOKEN", s.access_token.as_deref());

    let token = s.access_token.as_deref().unwrap_or("COLE_AQUI_O_TOKEN_DE_ACESSO");
    let server_name: String = format!("supabase-{}", info.account_alias.to_lowercase())
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    let mcp = format!(
        "{{\n  \"mcpServers\": {{\n    \"{server_name}\": {{\n      \"command\": \"npx\",\n      \"args\": [\"-y\", \"@supabase/mcp-server-supabase@latest\", \"--project-ref={r}\"],\n      \"env\": {{ \"SUPABASE_ACCESS_TOKEN\": \"{token}\" }}\n    }}\n  }}\n}}",
        r = s.project_ref
    );

    let mut notes = vec![
        "SUPABASE_SERVICE_ROLE_KEY, SUPABASE_SECRET_KEY e SUPABASE_JWT_SECRET passam por cima do RLS: use só no servidor.".to_string(),
        "DATABASE_URL usa o pooler em modo transação (porta 6543). Com Prisma, acrescente ?pgbouncer=true no fim dela.".to_string(),
        "DIRECT_URL usa o pooler em modo sessão (porta 5432): use para migrações.".to_string(),
        "Plano grátis: o projeto pausa depois de 7 dias sem uso, e cada conta tem só 2 projetos grátis ativos.".to_string(),
    ];
    if s.ipv6_only {
        notes.push("A conexão direta (SUPABASE_DB_DIRECT_URL) só funciona em rede com IPv6; sem IPv6, use as do pooler.".into());
    }
    if s.db_password.is_none() {
        notes.push(format!(
            "A senha do banco não está guardada: os endereços estão com {PASSWORD_PLACEHOLDER}. Peça a senha ao dono."
        ));
    }
    if s.access_token.is_none() {
        notes.push(
            "Falta o token de acesso (sem ele não dá para usar a API de gerenciamento nem o MCP): no Ultrabase, use \"Criar token para IA\" na conta."
                .into(),
        );
    }

    let org = match (info.organization_name, info.plan) {
        (Some(o), Some(p)) => format!("{o} (plano {p})"),
        (Some(o), None) => o.to_string(),
        _ => "desconhecida".into(),
    };
    let email = if info.account_email.is_empty() { "e-mail não lido" } else { info.account_email };

    // Rotas conferidas na especificação oficial (https://api.supabase.com/api/v1-json, 09/11/2026).
    let api = format!(
        "É a mesma API que o MCP do Supabase usa por baixo: com o token você faz tudo o que ele faz, sem MCP.\n\
         Em todo pedido: cabeçalho `Authorization: Bearer $SUPABASE_ACCESS_TOKEN`. Base: `https://api.supabase.com/v1/projects/{r}`\n\n\
         - Rodar SQL (lê e escreve, sem precisar da senha do banco): `POST /database/query` com `{{\"query\": \"...\"}}`. Só leitura: acrescente `\"read_only\": true`.\n\
         - Migração com nome, que fica no histórico: `POST /database/migrations` com `{{\"name\": \"...\", \"query\": \"...\"}}`. Listar: `GET /database/migrations`.\n\
         - Tipos TypeScript do banco: `GET /types/typescript`.\n\
         - Avisos de segurança e desempenho: `GET /advisors/security` e `GET /advisors/performance`.\n\
         - Logs: `GET /analytics/endpoints/logs.all`.\n\
         - Edge Functions: listar `GET /functions`; publicar `POST /functions/deploy?slug=<nome>` (multipart com `file` e `metadata`).\n\
         - Segredos das Edge Functions: `GET /secrets` e `POST /secrets`.\n\
         - Configuração de login (Auth): `GET /config/auth` e `PATCH /config/auth`.\n\
         - Chaves do projeto: `GET /api-keys?reveal=true`.\n\n\
         Exemplo:\n\n\
         ```bash\n\
         curl -X POST \"https://api.supabase.com/v1/projects/{r}/database/query\" \\\n  \
         -H \"Authorization: Bearer $SUPABASE_ACCESS_TOKEN\" -H \"Content-Type: application/json\" \\\n  \
         -d '{{\"query\": \"select count(*) from auth.users\"}}'\n\
         ```\n\n\
         O token enxerga todos os projetos da conta: use sempre o ref {r}.",
        r = s.project_ref
    );

    format!(
        "# Supabase: {name}\n\n\
         - Conta: {email} (no Ultrabase: {alias})\n\
         - Organização: {org}\n\
         - Projeto: {name}, ref {r}, região {region}, situação {status}\n\
         - Painel: {dash}\n\n\
         ## Variáveis de ambiente (.env)\n\n```env\n{env}```\n\n\
         ## Operar o projeto sem MCP (API do Supabase com o token)\n\n{api}\n\n\
         ## MCP do Supabase (opcional)\n\n\
         Só se quiser um MCP para esta conta. Se a IDE já tem um MCP do Supabase de outra conta, não troque: \
         dá para ter vários, cada um com um nome (este se chama \"{server_name}\"), ou usar só a API acima. \
         No Windows, se o npx não abrir, troque \"command\" por \"cmd\" e comece os args com \"/c\", \"npx\".\n\n\
         ```json\n{mcp}\n```\n\n\
         ## Avisos para a IA\n\n{notes}\n",
        name = s.name,
        alias = info.account_alias,
        r = s.project_ref,
        region = s.region,
        status = s.status,
        dash = s.dashboard_url,
        notes = notes.iter().map(|n| format!("- {n}")).collect::<Vec<_>>().join("\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> ProjectInfo<'static> {
        ProjectInfo {
            project_ref: "abcdefghijklmnopqrst",
            name: "loja",
            region: "us-east-1",
            status: "ACTIVE_HEALTHY",
            organization_name: Some("Minha org"),
            plan: Some("free"),
            account_alias: "Conta 1",
            account_email: "dono@exemplo.com",
        }
    }

    fn pooler() -> PoolerConfig {
        PoolerConfig {
            database_type: Some("PRIMARY".into()),
            db_user: "postgres.abcdefghijklmnopqrst".into(),
            db_host: "aws-0-us-east-1.pooler.supabase.com".into(),
            db_port: 6543,
            db_name: "postgres".into(),
        }
    }

    fn keys() -> ProjectKeySet {
        ProjectKeySet {
            anon: Some("anon-jwt".into()),
            publishable: Some("sb_publishable_x".into()),
            service_role: Some("service-jwt".into()),
            secret: Some("sb_secret_x".into()),
        }
    }

    #[test]
    fn password_goes_into_every_url_encoded() {
        let s = assemble(info(), keys(), None, Some(pooler()), Some("a@b:c/d e".into()), None);
        assert_eq!(s.direct_url, "postgresql://postgres:a%40b%3Ac%2Fd%20e@db.abcdefghijklmnopqrst.supabase.co:5432/postgres");
        assert_eq!(
            s.pooler_transaction_url.as_deref(),
            Some("postgresql://postgres.abcdefghijklmnopqrst:a%40b%3Ac%2Fd%20e@aws-0-us-east-1.pooler.supabase.com:6543/postgres")
        );
        assert!(s.pooler_session_url.unwrap().ends_with("pooler.supabase.com:5432/postgres"));
    }

    #[test]
    fn missing_password_uses_supabase_placeholder_and_warns() {
        let s = assemble(info(), keys(), None, Some(pooler()), None, None);
        assert!(s.direct_url.contains(":[YOUR-PASSWORD]@"));
        assert!(s.bundle.contains("SUPABASE_DB_PASSWORD=[YOUR-PASSWORD]"));
        assert!(s.bundle.contains("Peça a senha ao dono"));
        assert!(s.bundle.contains("COLE_AQUI_O_TOKEN_DE_ACESSO"));
    }

    #[test]
    fn bundle_carries_every_key_and_connection() {
        let settings = ProjectSettings {
            db_ip_addr_config: Some("ipv6".into()),
            jwt_secret: Some("jwt-segredo".into()),
            ..Default::default()
        };
        let s = assemble(info(), keys(), Some(settings), Some(pooler()), Some("senha".into()), Some("sbp_token".into()));
        for expected in [
            "SUPABASE_URL=https://abcdefghijklmnopqrst.supabase.co",
            "SUPABASE_ANON_KEY=anon-jwt",
            "SUPABASE_PUBLISHABLE_KEY=sb_publishable_x",
            "SUPABASE_SERVICE_ROLE_KEY=service-jwt",
            "SUPABASE_SECRET_KEY=sb_secret_x",
            "SUPABASE_JWT_SECRET=jwt-segredo",
            "SUPABASE_DB_PASSWORD=senha",
            "DATABASE_URL=postgresql://postgres.abcdefghijklmnopqrst:senha@aws-0-us-east-1.pooler.supabase.com:6543/postgres",
            "DIRECT_URL=postgresql://postgres.abcdefghijklmnopqrst:senha@aws-0-us-east-1.pooler.supabase.com:5432/postgres",
            "\"SUPABASE_ACCESS_TOKEN\": \"sbp_token\"",
            "--project-ref=abcdefghijklmnopqrst",
            "só funciona em rede com IPv6",
            "SUPABASE_PROJECT_REF=abcdefghijklmnopqrst",
            "SUPABASE_ACCESS_TOKEN=sbp_token",
            "## Operar o projeto sem MCP",
            "https://api.supabase.com/v1/projects/abcdefghijklmnopqrst/database/query",
            "\"supabase-conta-1\"",
            "não troque",
        ] {
            assert!(s.bundle.contains(expected), "faltou no pacote: {expected}");
        }
        assert!(!s.bundle.contains("[YOUR-PASSWORD]"));
    }

    /// Mostra o texto completo com dados falsos: `cargo test --lib mostra_exemplo -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn mostra_exemplo() {
        let settings = ProjectSettings { db_ip_addr_config: Some("ipv6".into()), jwt_secret: Some("jwt".into()), ..Default::default() };
        let s = assemble(info(), keys(), Some(settings), Some(pooler()), Some("senha".into()), Some("sbp_token".into()));
        println!("{}", s.bundle);
    }

    #[test]
    fn without_pooler_falls_back_to_direct_connection() {
        let s = assemble(info(), ProjectKeySet::default(), None, None, Some("x".into()), None);
        assert!(s.bundle.contains("DATABASE_URL=postgresql://postgres:x@db.abcdefghijklmnopqrst.supabase.co:5432/postgres"));
        assert!(!s.bundle.contains("SUPABASE_SECRET_KEY="));
    }
}
