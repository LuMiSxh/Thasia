use crate::conversion::ConvertError;
use crate::state::StateError;
use serde::Serialize;
use specta::Type;
use std::sync::PoisonError;
use thasia_core::prelude::ThasiaError;
use thasia_source::SourceError;
use thiserror::Error;

pub type CommandResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Type, Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppError {
    #[error("{message}")]
    Core {
        code: String,
        message: String,
        severity: ErrorSeverity,
        causes: Vec<String>,
    },
    #[error("{message}")]
    Convert {
        code: String,
        message: String,
        severity: ErrorSeverity,
        causes: Vec<String>,
    },
    #[error("{message}")]
    Source {
        code: String,
        message: String,
        severity: ErrorSeverity,
        causes: Vec<String>,
    },
    #[error("I/O error: {message}")]
    Io { message: String },
    #[error("JSON error: {message}")]
    Json { message: String },
    #[error("Semaphore error: {message}")]
    Semaphore { message: String },
    #[error("Task error: {message}")]
    Task { message: String },
    #[error("Open path error: {message}")]
    OpenPath { message: String },
    #[error("State lock failed: {message}")]
    StateLock { message: String },
    #[error("Suwayomi-Server is not installed")]
    SuwayomiNotInstalled,
    #[error("Suwayomi-Server is not running")]
    SuwayomiNotRunning,
    #[error("Suwayomi-Server is still starting")]
    SuwayomiStarting,
    #[error("Suwayomi-Server is not running: {message}")]
    SuwayomiRuntime { message: String },
    #[error("Suwayomi-Server is not ready")]
    SuwayomiNotReady,
    #[error("Cancelled")]
    Cancelled,
    #[error("{message}")]
    Message { message: String },
}

#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Warning,
    Recoverable,
    Fatal,
}

impl From<ThasiaError> for AppError {
    fn from(error: ThasiaError) -> Self {
        Self::Core {
            code: core_error_code(&error).to_string(),
            message: error.to_string(),
            severity: core_error_severity(&error),
            causes: error_causes(&error),
        }
    }
}

impl From<ConvertError> for AppError {
    fn from(error: ConvertError) -> Self {
        Self::Convert {
            code: error.code().to_string(),
            message: error.to_string(),
            severity: error.severity(),
            causes: error_causes(&error),
        }
    }
}

impl From<SourceError> for AppError {
    fn from(error: SourceError) -> Self {
        let code = match &error {
            SourceError::Io(_) => "thasia::source::io",
            SourceError::Join(_) => "thasia::source::task",
            SourceError::Http(_) => "thasia::source::http",
            SourceError::Json(_) => "thasia::source::json",
            SourceError::Zip(_) => "thasia::source::zip",
            SourceError::Archive(_) => "thasia::source::archive",
            SourceError::Suwayomi(_) => "thasia::source::suwayomi",
        };
        Self::Source {
            code: code.to_string(),
            message: error.to_string(),
            severity: ErrorSeverity::Recoverable,
            causes: error_causes(&error),
        }
    }
}

impl From<StateError> for AppError {
    fn from(error: StateError) -> Self {
        match error {
            StateError::Io(error) => error.into(),
            StateError::Json(error) => error.into(),
            StateError::Core(error) => error.into(),
            StateError::Source(error) => error.into(),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json {
            message: error.to_string(),
        }
    }
}

impl From<tokio::sync::AcquireError> for AppError {
    fn from(error: tokio::sync::AcquireError) -> Self {
        Self::Semaphore {
            message: error.to_string(),
        }
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::Task {
            message: error.to_string(),
        }
    }
}

impl From<tauri_plugin_opener::Error> for AppError {
    fn from(error: tauri_plugin_opener::Error) -> Self {
        Self::OpenPath {
            message: error.to_string(),
        }
    }
}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        Self::Message { message }
    }
}

impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        Self::Message {
            message: message.to_string(),
        }
    }
}

impl<T> From<PoisonError<T>> for AppError {
    fn from(error: PoisonError<T>) -> Self {
        Self::StateLock {
            message: error.to_string(),
        }
    }
}

fn core_error_code(error: &ThasiaError) -> &'static str {
    match error {
        ThasiaError::ItemProcessFailed { .. } => "thasia::item::process_failed",
        ThasiaError::VolumeSkipped { .. } => "thasia::volume::skipped",
        ThasiaError::UnresolvedPath { .. } => "thasia::parse::unresolved",
        ThasiaError::EmptyFilename => "thasia::filename::empty",
        ThasiaError::InvalidFilenameComponent { .. } => "thasia::filename::invalid_component",
        ThasiaError::FilenameTrailingDotOrSpace { .. } => "thasia::filename::trailing_dot_or_space",
        ThasiaError::UnsafeFilenameCharacter { .. } => "thasia::filename::unsafe_character",
        ThasiaError::WindowsReservedFilename { .. } => "thasia::filename::windows_reserved",
        ThasiaError::Fatal(_) => "thasia::fatal::pipeline_aborted",
        ThasiaError::Discovery(_) => "thasia::discovery",
        ThasiaError::Io(_) => "thasia::io",
    }
}

fn core_error_severity(error: &ThasiaError) -> ErrorSeverity {
    match error {
        ThasiaError::ItemProcessFailed { .. } => ErrorSeverity::Warning,
        ThasiaError::VolumeSkipped { .. } | ThasiaError::UnresolvedPath { .. } => {
            ErrorSeverity::Recoverable
        }
        ThasiaError::EmptyFilename
        | ThasiaError::InvalidFilenameComponent { .. }
        | ThasiaError::FilenameTrailingDotOrSpace { .. }
        | ThasiaError::UnsafeFilenameCharacter { .. }
        | ThasiaError::WindowsReservedFilename { .. } => ErrorSeverity::Recoverable,
        _ => ErrorSeverity::Fatal,
    }
}

fn error_causes(error: &dyn std::error::Error) -> Vec<String> {
    std::iter::successors(error.source(), |source| source.source())
        .map(|source| source.to_string())
        .collect()
}
