use thasia_core::ThasiaError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, SourceError>;

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Archive error: {0}")]
    Archive(String),
    #[error("Suwayomi error: {0}")]
    Suwayomi(String),
}

impl SourceError {
    pub fn archive(message: impl Into<String>) -> Self {
        Self::Archive(message.into())
    }

    pub fn suwayomi(message: impl Into<String>) -> Self {
        Self::Suwayomi(message.into())
    }
}

impl From<SourceError> for ThasiaError {
    fn from(error: SourceError) -> Self {
        match error {
            SourceError::Io(error) => Self::Io(error),
            error => Self::Discovery(error.to_string()),
        }
    }
}
