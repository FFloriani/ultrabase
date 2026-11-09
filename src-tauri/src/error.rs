use crate::supabase::ApiError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("banco local: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("cofre do Windows: {0}")]
    Vault(#[from] keyring::Error),
    #[error("{0}")]
    Api(#[from] ApiError),
    #[error("janela: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("arquivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("conta não encontrada")]
    AccountNotFound,
}

impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
