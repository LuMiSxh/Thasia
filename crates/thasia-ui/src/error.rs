use std::path::PathBuf;
use thasia_core::prelude::ThasiaError;
use thasia_packager::PackagerError;
use thasia_processor::ProcessorError;
use thasia_source::SourceError;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Core(#[from] ThasiaError),
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("No scan result. Scan a source first.")]
    MissingScanResult,
    #[error("The scanned source is no longer available.")]
    MissingSource,
    #[error("Conversion was cancelled")]
    Cancelled,
    #[error("Output path was not initialized")]
    MissingOutputPath,
    #[error("Volume {volume_num} completed with missing pages ({actual}/{expected})")]
    IncompleteVolume {
        volume_num: u32,
        expected: u32,
        actual: u32,
    },
    #[error("Failed to create output directory {path:?}: {source}")]
    CreateOutputDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Custom page is not a supported image: {path:?}")]
    UnsupportedCustomImage { path: PathBuf },
    #[error("Custom page is not readable ({path:?}): {source}")]
    CustomImageMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Custom page is not a file: {path:?}")]
    CustomImageNotFile { path: PathBuf },
    #[error("Custom page is too large ({bytes} bytes): {path:?}")]
    CustomImageTooLarge { path: PathBuf, bytes: u64 },
    #[error(transparent)]
    Package(#[from] PackagerError),
    #[error(transparent)]
    Pipeline(#[from] ProcessorError),
    #[error("{0}")]
    Message(String),
}

impl From<&str> for AppError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}
