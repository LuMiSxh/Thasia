//! Raw directory generator — dumps processed images to a flat directory.

use crate::Generator;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use thasia_core::{models::ProcessedImage, Result, ThasiaError};

pub struct RawGenerator {
    output_dir: PathBuf,
}

impl RawGenerator {
    pub fn new() -> Self {
        Self { output_dir: PathBuf::new() }
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
        self.output_dir = output_dir.join(volume_name);
        tokio::fs::create_dir_all(&self.output_dir)
            .await
            .map_err(ThasiaError::Io)?;
        Ok(())
    }

    async fn add_page(&mut self, img: ProcessedImage) -> Result<()> {
        let filename = format!("page_{:04}.{}", img.parsed_data.page_number, img.ext);
        let dest = self.output_dir.join(filename);
        tokio::fs::write(dest, &img.image_data)
            .await
            .map_err(ThasiaError::Io)?;
        Ok(())
    }

    async fn finalize(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}
