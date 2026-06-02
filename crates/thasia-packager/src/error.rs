use thasia_core::ThasiaError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, PackagerError>;

#[derive(Debug, Error)]
pub enum PackagerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Core error: {0}")]
    Core(#[from] ThasiaError),
    #[error("ZIP package error: {0}")]
    Zip(#[from] async_zip::error::ZipError),
    #[error("EPUB package error: {0}")]
    Epub(String),
    #[error("Packager is not initialized: {0}")]
    NotInitialized(&'static str),
    #[error("Background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
}

impl PackagerError {
    pub fn epub(error: impl std::fmt::Display) -> Self {
        Self::Epub(error.to_string())
    }
}

impl From<PackagerError> for ThasiaError {
    fn from(error: PackagerError) -> Self {
        match error {
            PackagerError::Io(error) => Self::Io(error),
            error => Self::Fatal(error.to_string()),
        }
    }
}
