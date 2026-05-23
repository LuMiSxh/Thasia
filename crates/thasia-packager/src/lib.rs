pub mod cbz;
pub mod epub;
pub mod raw;

pub use cbz::CbzGenerator;
pub use epub::EpubGenerator;
pub use raw::RawGenerator;

use async_trait::async_trait;
use std::path::Path;
use thasia_core::{Result, models::ProcessedImage};

#[async_trait]
pub trait Generator: Send {
    async fn init(&mut self, output_dir: &Path, volume_name: &str) -> Result<()>;
    async fn add_page(&mut self, image: ProcessedImage) -> Result<()>;
    async fn finalize(self: Box<Self>) -> Result<()>;
}
