use thasia_core::ThasiaError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ProcessorError>;

#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("Image decode failed: {0}")]
    Decode(String),
    #[error("AVIF decode failed: {0}")]
    AvifDecode(String),
    #[error("Invalid decoded image buffer: {0}")]
    InvalidBuffer(&'static str),
    #[error("Unsupported encode format: {0}")]
    UnsupportedFormat(&'static str),
    #[error("AVIF encode failed: {0}")]
    AvifEncode(String),
    #[error("WebP encode failed: {0}")]
    WebpEncode(String),
}

impl ProcessorError {
    pub fn decode(error: impl std::fmt::Display) -> Self {
        Self::Decode(error.to_string())
    }

    pub fn avif_decode(error: impl std::fmt::Display) -> Self {
        Self::AvifDecode(error.to_string())
    }

    pub fn avif_encode(error: impl std::fmt::Display) -> Self {
        Self::AvifEncode(error.to_string())
    }

    pub fn webp_encode(error: impl std::fmt::Display) -> Self {
        Self::WebpEncode(error.to_string())
    }
}

impl From<ProcessorError> for ThasiaError {
    fn from(error: ProcessorError) -> Self {
        Self::Fatal(error.to_string())
    }
}
