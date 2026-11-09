//! Segredos de cada conta no Gerenciador de Credenciais do Windows.
//! Nunca há plano B em arquivo: se o cofre falhar, o erro sobe para a tela.
//! Nomes no cofre (serviço "ultrabase"): "<conta>:password", "<conta>:token" e "<conta>:db:<ref>"
//! (a senha do banco de cada projeto, que o Supabase não guarda e por isso só o app pode lembrar).

use keyring::Entry;

use crate::error::AppResult;

const SERVICE: &str = "ultrabase";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secret {
    Password,
    Token,
}

impl Secret {
    pub fn parse(kind: &str) -> Option<Self> {
        match kind {
            "password" => Some(Secret::Password),
            "token" => Some(Secret::Token),
            _ => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Secret::Password => "password",
            Secret::Token => "token",
        }
    }
}

fn entry(account_id: &str, key: &str) -> AppResult<Entry> {
    Ok(Entry::new(SERVICE, &format!("{account_id}:{key}"))?)
}

fn get_raw(account_id: &str, key: &str) -> AppResult<Option<String>> {
    match entry(account_id, key)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Texto vazio apaga.
fn set_raw(account_id: &str, key: &str, value: &str) -> AppResult<()> {
    let value = value.trim();
    if value.is_empty() {
        return delete_raw(account_id, key);
    }
    entry(account_id, key)?.set_password(value)?;
    Ok(())
}

fn delete_raw(account_id: &str, key: &str) -> AppResult<()> {
    match entry(account_id, key)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn db_key(project_ref: &str) -> String {
    format!("db:{project_ref}")
}

pub fn get(account_id: &str, secret: Secret) -> AppResult<Option<String>> {
    get_raw(account_id, secret.key())
}

pub fn has(account_id: &str, secret: Secret) -> bool {
    matches!(get(account_id, secret), Ok(Some(_)))
}

pub fn set(account_id: &str, secret: Secret, value: &str) -> AppResult<()> {
    set_raw(account_id, secret.key(), value)
}

pub fn get_db_password(account_id: &str, project_ref: &str) -> AppResult<Option<String>> {
    get_raw(account_id, &db_key(project_ref))
}

pub fn set_db_password(account_id: &str, project_ref: &str, value: &str) -> AppResult<()> {
    set_raw(account_id, &db_key(project_ref), value)
}

/// Apaga tudo da conta, inclusive a senha do banco de cada projeto dela.
pub fn delete_all(account_id: &str, project_refs: &[String]) -> AppResult<()> {
    delete_raw(account_id, Secret::Password.key())?;
    delete_raw(account_id, Secret::Token.key())?;
    for project_ref in project_refs {
        delete_raw(account_id, &db_key(project_ref))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mexe no cofre de verdade do Windows; roda com `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn roundtrip_in_windows_credential_manager() {
        let id = format!("teste-{}", uuid::Uuid::new_v4());
        set(&id, Secret::Token, "sbp_teste").unwrap();
        assert_eq!(get(&id, Secret::Token).unwrap().as_deref(), Some("sbp_teste"));
        set(&id, Secret::Token, "  ").unwrap();
        assert_eq!(get(&id, Secret::Token).unwrap(), None);
        set_db_password(&id, "abcdefghijklmnopqrst", "senha-do-banco").unwrap();
        assert_eq!(get_db_password(&id, "abcdefghijklmnopqrst").unwrap().as_deref(), Some("senha-do-banco"));
        delete_all(&id, &["abcdefghijklmnopqrst".to_string()]).unwrap();
        assert_eq!(get_db_password(&id, "abcdefghijklmnopqrst").unwrap(), None);
    }
}
