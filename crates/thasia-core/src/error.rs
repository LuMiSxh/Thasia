use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum ThasiaError {
    // --- LEVEL 1: Item Level (Retryable) ---
    #[error("Failed to process image after retries: {file}")]
    #[diagnostic(code(thasia::item::process_failed))]
    ItemProcessFailed { file: String, source: std::io::Error },

    // --- LEVEL 2: Volume Level (Skippable) ---
    #[error("Volume skipped due to critical failure: {volume}")]
    #[diagnostic(
        code(thasia::volume::skipped),
        help("Verify directory permissions or archive integrity.")
    )]
    VolumeSkipped { volume: String, reason: String },

    #[error("Failed to parse path: {path}")]
    #[diagnostic(code(thasia::parse::unresolved))]
    UnresolvedPath { path: String },

    // --- LEVEL 3: Fatal Level (Abort) ---
    #[error("Fatal Pipeline Error: {0}")]
    #[diagnostic(code(thasia::fatal::pipeline_aborted))]
    Fatal(String),

    #[error("I/O Error: {0}")]
    #[diagnostic(code(thasia::fatal::io))]
    Io(#[from] std::io::Error),
}

/// A structured error representation that can be sent to a frontend via Tauri
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tauri", derive(specta::Type))]
pub struct SerializableError {
    pub code: String,
    pub message: String,
    pub severity: u8, // 1 = Warning, 2 = Skip, 3 = Fatal
}

impl From<&ThasiaError> for SerializableError {
    fn from(err: &ThasiaError) -> Self {
        use miette::Diagnostic as _;
        let code = err.code().map(|c| c.to_string()).unwrap_or_else(|| "thasia::unknown".into());
        let severity = match err {
            ThasiaError::ItemProcessFailed { .. } => 1,
            ThasiaError::VolumeSkipped { .. } | ThasiaError::UnresolvedPath { .. } => 2,
            _ => 3,
        };
        Self { code, message: err.to_string(), severity }
    }
}

pub type Result<T> = std::result::Result<T, ThasiaError>;
