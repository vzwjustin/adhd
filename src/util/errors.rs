use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnchorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Repo error: {0}")]
    Repo(String),

    #[error("Thread error: {0}")]
    Thread(String),

    #[error("No active session")]
    NoActiveSession,

    #[error("No active thread")]
    NoActiveThread,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AnchorError>;
