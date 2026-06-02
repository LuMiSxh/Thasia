mod events;
mod page_edits;
mod runner;
mod volume;

use std::path::PathBuf;
use std::sync::PoisonError;
use thasia_core::prelude::ThasiaError;
use thasia_packager::PackagerError;
use thasia_processor::ProcessorError;
use thiserror::Error;

use crate::app_error::ErrorSeverity;

pub(crate) use events::{ConversionEvents, TauriConversionEvents};
pub(crate) use page_edits::resolve_edit_pages;
pub(crate) use runner::run_tauri_conversion;
pub(crate) use volume::{ConversionStats, PackageSelection, convert_volume, duration_ms};

pub(crate) type ConvertResult<T> = Result<T, ConvertError>;

#[derive(Debug, Error)]
pub(crate) enum ConvertError {
    #[error("No scan result - run scan_source first")]
    MissingScanResult,
    #[error("Source missing - re-run scan_source")]
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
    #[error("State lock failed: {0}")]
    StateLock(String),
    #[error("Failed to create output directory {path:?}: {source}")]
    CreateOutputDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Invalid output name: {0}")]
    InvalidOutputName(#[from] ThasiaError),
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
    #[error("Package error: {0}")]
    Package(#[from] PackagerError),
    #[error("Image pipeline error: {0}")]
    Pipeline(#[from] ProcessorError),
}

impl ConvertError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            ConvertError::MissingScanResult => "thasia::convert::missing_scan_result",
            ConvertError::MissingSource => "thasia::convert::missing_source",
            ConvertError::Cancelled => "thasia::convert::cancelled",
            ConvertError::MissingOutputPath => "thasia::convert::missing_output_path",
            ConvertError::IncompleteVolume { .. } => "thasia::convert::incomplete_volume",
            ConvertError::StateLock(_) => "thasia::state::lock",
            ConvertError::CreateOutputDir { .. } => "thasia::convert::create_output_dir",
            ConvertError::InvalidOutputName(_) => "thasia::convert::invalid_output_name",
            ConvertError::UnsupportedCustomImage { .. } => "thasia::convert::custom_image_type",
            ConvertError::CustomImageMetadata { .. } => "thasia::convert::custom_image_metadata",
            ConvertError::CustomImageNotFile { .. } => "thasia::convert::custom_image_not_file",
            ConvertError::CustomImageTooLarge { .. } => "thasia::convert::custom_image_too_large",
            ConvertError::Package(_) => "thasia::package",
            ConvertError::Pipeline(_) => "thasia::pipeline",
        }
    }

    pub(crate) fn severity(&self) -> ErrorSeverity {
        match self {
            ConvertError::Cancelled
            | ConvertError::MissingScanResult
            | ConvertError::MissingSource
            | ConvertError::MissingOutputPath
            | ConvertError::IncompleteVolume { .. }
            | ConvertError::UnsupportedCustomImage { .. }
            | ConvertError::CustomImageNotFile { .. }
            | ConvertError::CustomImageTooLarge { .. } => ErrorSeverity::Recoverable,
            _ => ErrorSeverity::Fatal,
        }
    }
}

impl<T> From<PoisonError<T>> for ConvertError {
    fn from(error: PoisonError<T>) -> Self {
        Self::StateLock(error.to_string())
    }
}
