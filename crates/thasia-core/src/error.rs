use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum ThasiaError {
    // --- LEVEL 1: Item Level (Retryable) ---
    #[error("Failed to process image after retries: {file}")]
    #[diagnostic(code(thasia::item::process_failed))]
    ItemProcessFailed {
        file: String,
        source: std::io::Error,
    },

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

    #[error("Output name cannot be empty")]
    #[diagnostic(code(thasia::filename::empty))]
    EmptyFilename,

    #[error("Invalid output name component: {value}")]
    #[diagnostic(code(thasia::filename::invalid_component))]
    InvalidFilenameComponent { value: String },

    #[error("Output name cannot end with a dot or space: {value}")]
    #[diagnostic(code(thasia::filename::trailing_dot_or_space))]
    FilenameTrailingDotOrSpace { value: String },

    #[error("Output name contains characters that are not safe for filenames: {value}")]
    #[diagnostic(code(thasia::filename::unsafe_character))]
    UnsafeFilenameCharacter { value: String },

    #[error("Output name is reserved on Windows: {value}")]
    #[diagnostic(code(thasia::filename::windows_reserved))]
    WindowsReservedFilename { value: String },

    // --- LEVEL 3: Fatal Level (Abort) ---
    #[error("Fatal Pipeline Error: {0}")]
    #[diagnostic(code(thasia::fatal::pipeline_aborted))]
    Fatal(String),

    #[error("Discovery Error: {0}")]
    #[diagnostic(code(thasia::discovery::suwayomi))]
    Discovery(String),

    #[error("I/O Error: {0}")]
    #[diagnostic(code(thasia::fatal::io))]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ThasiaError>;
