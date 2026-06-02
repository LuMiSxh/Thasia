//! Raw directory generator — dumps processed images to a flat directory.

use crate::{Generator, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use thasia_core::{models::ProcessedImage, sanitize_filename_component};

pub struct RawGenerator {
    output_dir: PathBuf,
}

impl RawGenerator {
    pub fn new() -> Self {
        Self {
            output_dir: PathBuf::new(),
        }
    }
}

impl Default for RawGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for RawGenerator {
    async fn init(&mut self, output_dir: &Path, volume_name: &str) -> Result<()> {
        let safe_volume_name = sanitize_filename_component(volume_name)?;
        self.output_dir = output_dir.join(safe_volume_name);
        tokio::fs::create_dir_all(&self.output_dir).await?;
        Ok(())
    }

    async fn add_page(&mut self, img: ProcessedImage) -> Result<()> {
        let filename = format!(
            "page_{:04}.{}",
            (img.parsed_data.page_number as u32) + 1,
            img.ext
        );
        let dest = self.output_dir.join(filename);
        tokio::fs::write(dest, &img.image_data).await?;
        Ok(())
    }

    fn output_path(&self) -> Option<PathBuf> {
        Some(self.output_dir.clone())
    }

    async fn finalize(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}
