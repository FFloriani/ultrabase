//! Banco local (SQLite): contas e a última cópia de organizações e projetos.
//! Segredos não entram aqui; ficam no cofre do Windows.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::models::{Account, Organization, Project, SessionStatus, TokenStatus};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS accounts (
    id              TEXT PRIMARY KEY,
    alias           TEXT NOT NULL,
    color           TEXT NOT NULL,
    email           TEXT NOT NULL DEFAULT '',
    login_method    TEXT NOT NULL DEFAULT 'email',
    notes           TEXT NOT NULL DEFAULT '',
    username        TEXT,
    token_status    TEXT NOT NULL DEFAULT 'missing',
    last_sync_at    TEXT,
    last_sync_error TEXT,
    created_at      TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS organizations (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    id         TEXT NOT NULL,
    slug       TEXT NOT NULL,
    name       TEXT NOT NULL,
    plan       TEXT,
    PRIMARY KEY (account_id, id)
);
CREATE TABLE IF NOT EXISTS projects (
    account_id       TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    ref              TEXT NOT NULL,
    name             TEXT NOT NULL,
    region           TEXT NOT NULL,
    organization_id  TEXT NOT NULL,
    status           TEXT NOT NULL,
    created_at       TEXT,
    last_activity_at TEXT,
    usage_checked    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (account_id, ref)
);
"#;

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    prepare(conn)
}

#[cfg(test)]
pub fn open_in_memory() -> rusqlite::Result<Connection> {
    prepare(Connection::open_in_memory()?)
}

/// Mudanças no banco depois da primeira versão, em ordem. A posição + 1 é o `user_version`.
const MIGRATIONS: &[&str] = &[
    // 1: estado do login do Supabase no navegador da conta.
    "ALTER TABLE accounts ADD COLUMN session_status TEXT NOT NULL DEFAULT 'none';",
];

fn prepare(conn: Connection) -> rusqlite::Result<Connection> {
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.execute_batch(SCHEMA)?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: usize = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
        conn.execute_batch(sql)?;
        conn.pragma_update(None, "user_version", (i + 1) as i64)?;
    }
    Ok(())
}

const ACCOUNT_COLUMNS: &str = "id, alias, color, email, login_method, notes, username, token_status, \
     last_sync_at, last_sync_error, created_at, session_status";

fn account_from_row(row: &Row) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        alias: row.get(1)?,
        color: row.get(2)?,
        email: row.get(3)?,
        login_method: row.get(4)?,
        notes: row.get(5)?,
        username: row.get(6)?,
        token_status: TokenStatus::parse(&row.get::<_, String>(7)?),
        last_sync_at: row.get(8)?,
        last_sync_error: row.get(9)?,
        created_at: row.get(10)?,
        session_status: SessionStatus::parse(&row.get::<_, String>(11)?),
    })
}

pub fn list_accounts(conn: &Connection) -> rusqlite::Result<Vec<Account>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {ACCOUNT_COLUMNS} FROM accounts ORDER BY created_at, alias"
    ))?;
    let rows = stmt.query_map([], account_from_row)?;
    rows.collect()
}

pub fn get_account(conn: &Connection, id: &str) -> rusqlite::Result<Option<Account>> {
    conn.query_row(
        &format!("SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE id = ?1"),
        [id],
        account_from_row,
    )
    .optional()
}

pub fn account_ids(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT id FROM accounts")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect()
}

pub fn insert_account(conn: &Connection, a: &Account) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO accounts (id, alias, color, email, login_method, notes, token_status, created_at, session_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            a.id,
            a.alias,
            a.color,
            a.email,
            a.login_method,
            a.notes,
            a.token_status.as_str(),
            a.created_at,
            a.session_status.as_str()
        ],
    )?;
    Ok(())
}

pub fn aliases(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT alias FROM accounts")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect()
}

pub fn set_session_status(conn: &Connection, id: &str, status: SessionStatus) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE accounts SET session_status = ?2 WHERE id = ?1",
        params![id, status.as_str()],
    )?;
    Ok(())
}

pub fn update_account_details(
    conn: &Connection,
    id: &str,
    alias: &str,
    color: &str,
    email: &str,
    login_method: &str,
    notes: &str,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "UPDATE accounts SET alias = ?2, color = ?3, email = ?4, login_method = ?5, notes = ?6 WHERE id = ?1",
        params![id, alias, color, email, login_method, notes],
    )?;
    Ok(changed > 0)
}

pub fn set_token_status(conn: &Connection, id: &str, status: TokenStatus) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE accounts SET token_status = ?2 WHERE id = ?1",
        params![id, status.as_str()],
    )?;
    Ok(())
}

pub fn delete_account(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM accounts WHERE id = ?1", [id])?;
    Ok(())
}

pub fn list_organizations(conn: &Connection, account_id: &str) -> rusqlite::Result<Vec<Organization>> {
    let mut stmt = conn.prepare(
        "SELECT id, slug, name, plan FROM organizations WHERE account_id = ?1 ORDER BY name",
    )?;
    let rows = stmt.query_map([account_id], |row| {
        Ok(Organization { id: row.get(0)?, slug: row.get(1)?, name: row.get(2)?, plan: row.get(3)? })
    })?;
    rows.collect()
}

pub fn list_projects(conn: &Connection, account_id: &str) -> rusqlite::Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT ref, name, region, organization_id, status, created_at, last_activity_at, usage_checked
         FROM projects WHERE account_id = ?1 ORDER BY created_at, name",
    )?;
    let rows = stmt.query_map([account_id], |row| {
        Ok(Project {
            project_ref: row.get(0)?,
            name: row.get(1)?,
            region: row.get(2)?,
            organization_id: row.get(3)?,
            status: row.get(4)?,
            created_at: row.get(5)?,
            last_activity_at: row.get(6)?,
            usage_checked: row.get::<_, i64>(7)? != 0,
        })
    })?;
    rows.collect()
}

pub struct SyncProfile<'a> {
    pub username: Option<&'a str>,
    /// Só preenche o e-mail da conta quando ele está vazio.
    pub primary_email: Option<&'a str>,
}

/// Troca a cópia inteira da conta de uma vez: ou grava tudo, ou nada.
pub fn save_sync(
    conn: &mut Connection,
    account_id: &str,
    profile: SyncProfile,
    organizations: &[Organization],
    projects: &[Project],
    synced_at: &str,
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM organizations WHERE account_id = ?1", [account_id])?;
    tx.execute("DELETE FROM projects WHERE account_id = ?1", [account_id])?;
    for o in organizations {
        tx.execute(
            "INSERT INTO organizations (account_id, id, slug, name, plan) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![account_id, o.id, o.slug, o.name, o.plan],
        )?;
    }
    for p in projects {
        tx.execute(
            "INSERT INTO projects (account_id, ref, name, region, organization_id, status, created_at,
                                   last_activity_at, usage_checked)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                account_id,
                p.project_ref,
                p.name,
                p.region,
                p.organization_id,
                p.status,
                p.created_at,
                p.last_activity_at,
                p.usage_checked as i64
            ],
        )?;
    }
    tx.execute(
        "UPDATE accounts SET username = ?2, last_sync_at = ?3, last_sync_error = NULL,
                email = CASE WHEN email = '' AND ?4 IS NOT NULL THEN ?4 ELSE email END
         WHERE id = ?1",
        params![account_id, profile.username, synced_at, profile.primary_email],
    )?;
    tx.commit()
}

pub fn save_sync_error(conn: &Connection, account_id: &str, error: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE accounts SET last_sync_error = ?2 WHERE id = ?1",
        params![account_id, error],
    )?;
    Ok(())
}

pub fn set_project_status(conn: &Connection, account_id: &str, project_ref: &str, status: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE projects SET status = ?3 WHERE account_id = ?1 AND ref = ?2",
        params![account_id, project_ref, status],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: &str) -> Account {
        Account {
            id: id.into(),
            alias: format!("conta {id}"),
            color: "#3ecf8e".into(),
            email: format!("{id}@exemplo.com"),
            login_method: "email".into(),
            notes: String::new(),
            username: None,
            session_status: SessionStatus::None,
            token_status: TokenStatus::Missing,
            last_sync_at: None,
            last_sync_error: None,
            created_at: format!("2026-11-09T00:00:0{id}Z"),
        }
    }

    #[test]
    fn migrates_database_from_first_version() {
        let path = std::env::temp_dir().join(format!("ultrabase-migracao-{}.db", uuid::Uuid::new_v4()));
        {
            let old = Connection::open(&path).unwrap();
            old.execute_batch(SCHEMA).unwrap();
            old.execute(
                "INSERT INTO accounts (id, alias, color, created_at) VALUES ('velha', 'Velha', '#62c9a5', 't')",
                [],
            )
            .unwrap();
        }
        let conn = open(&path).unwrap();
        let a = get_account(&conn, "velha").unwrap().unwrap();
        assert_eq!(a.session_status, SessionStatus::None);
        set_session_status(&conn, "velha", SessionStatus::Connected).unwrap();
        drop(conn);
        let again = open(&path).unwrap();
        assert_eq!(get_account(&again, "velha").unwrap().unwrap().session_status, SessionStatus::Connected);
        drop(again);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }

    fn project(r: &str, status: &str) -> Project {
        Project {
            project_ref: r.into(),
            name: r.into(),
            region: "sa-east-1".into(),
            organization_id: "org".into(),
            status: status.into(),
            created_at: None,
            last_activity_at: None,
            usage_checked: true,
        }
    }

    #[test]
    fn account_roundtrip_and_update() {
        let conn = open_in_memory().unwrap();
        insert_account(&conn, &account("1")).unwrap();
        assert!(update_account_details(&conn, "1", "pessoal", "#fff", "a@b.c", "github", "nota").unwrap());
        let a = get_account(&conn, "1").unwrap().unwrap();
        assert_eq!((a.alias.as_str(), a.login_method.as_str(), a.notes.as_str()), ("pessoal", "github", "nota"));
        assert!(!update_account_details(&conn, "nao-existe", "x", "x", "x", "email", "").unwrap());
    }

    #[test]
    fn sync_replaces_previous_copy() {
        let mut conn = open_in_memory().unwrap();
        insert_account(&conn, &account("1")).unwrap();
        let org = Organization { id: "org".into(), slug: "org".into(), name: "Org".into(), plan: Some("free".into()) };
        let profile = || SyncProfile { username: Some("user"), primary_email: Some("outro@exemplo.com") };
        save_sync(&mut conn, "1", profile(), std::slice::from_ref(&org), &[project("a", "ACTIVE_HEALTHY"), project("b", "INACTIVE")], "t1").unwrap();
        save_sync(&mut conn, "1", profile(), &[org], &[project("c", "ACTIVE_HEALTHY")], "t2").unwrap();
        let refs: Vec<_> = list_projects(&conn, "1").unwrap().into_iter().map(|p| p.project_ref).collect();
        assert_eq!(refs, vec!["c"]);
        let a = get_account(&conn, "1").unwrap().unwrap();
        assert_eq!(a.token_status, TokenStatus::Missing, "ler pela sessão não mexe no token");
        assert_eq!(a.last_sync_at.as_deref(), Some("t2"));
        assert_eq!(a.email, "1@exemplo.com", "e-mail já preenchido não muda");
    }

    #[test]
    fn sync_fills_empty_email_from_profile() {
        let mut conn = open_in_memory().unwrap();
        let mut a = account("1");
        a.email = String::new();
        insert_account(&conn, &a).unwrap();
        save_sync(&mut conn, "1", SyncProfile { username: None, primary_email: Some("p@exemplo.com") }, &[], &[], "t").unwrap();
        assert_eq!(get_account(&conn, "1").unwrap().unwrap().email, "p@exemplo.com");
    }

    #[test]
    fn deleting_account_removes_its_copy() {
        let mut conn = open_in_memory().unwrap();
        insert_account(&conn, &account("1")).unwrap();
        insert_account(&conn, &account("2")).unwrap();
        let none = || SyncProfile { username: None, primary_email: None };
        save_sync(&mut conn, "1", none(), &[], &[project("a", "ACTIVE_HEALTHY")], "t").unwrap();
        save_sync(&mut conn, "2", none(), &[], &[project("b", "ACTIVE_HEALTHY")], "t").unwrap();
        delete_account(&conn, "1").unwrap();
        assert!(list_projects(&conn, "1").unwrap().is_empty());
        assert_eq!(list_projects(&conn, "2").unwrap().len(), 1);
        assert_eq!(account_ids(&conn).unwrap(), vec!["2".to_string()]);
    }
}
