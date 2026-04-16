use crate::Generator;
use async_trait::async_trait;
use async_zip::{base::write::ZipFileWriter as BaseZipFileWriter, tokio::write::ZipFileWriter, Compression, ZipEntryBuilder};
use std::path::Path;
use thasia_core::{models::ProcessedImage, Result, ThasiaError};
use tokio::fs::File;

pub struct CbzGenerator {
    writer: Option<ZipFileWriter<File>>,
    page_count: u32,
}

impl CbzGenerator {
    pub fn new() -> Self {
        Self { writer: None, page_count: 0 }
    }
}

impl Default for CbzGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for CbzGenerator {
    async fn init(&mut self, output_dir: &Path, volume_name: &str) -> Result<()> {
        tokio::fs::create_dir_all(output_dir).await.map_err(ThasiaError::Io)?;
        let path = output_dir.join(format!("{volume_name}.cbz"));
        let file = File::create(&path).await.map_err(ThasiaError::Io)?;
        self.writer = Some(BaseZipFileWriter::with_tokio(file));
        Ok(())
    }

    async fn add_page(&mut self, img: ProcessedImage) -> Result<()> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| ThasiaError::Fatal("CbzGenerator not initialized".into()))?;

        let filename = format!("page_{:04}.{}", img.parsed_data.page_number, img.ext);
        // AVIF and WebP are already compressed — use Stored to avoid double compression.
        let compression = match img.ext.as_str() {
            "avif" | "webp" => Compression::Stored,
            _ => Compression::Deflate,
        };

        let builder = ZipEntryBuilder::new(filename.into(), compression);
        writer
            .write_entry_whole(builder, &img.image_data)
            .await
            .map_err(|e| ThasiaError::Fatal(e.to_string()))?;

        self.page_count += 1;
        Ok(())
    }

    async fn finalize(mut self: Box<Self>) -> Result<()> {
        let mut writer = self
            .writer
            .take()
            .ok_or_else(|| ThasiaError::Fatal("CbzGenerator not initialized".into()))?;

        // Write ComicInfo.xml — Anansi Project schema
        let xml = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<ComicInfo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
           xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <Manga>Yes</Manga>
  <PageCount>{}</PageCount>
</ComicInfo>"#,
            self.page_count
        );

        let builder = ZipEntryBuilder::new("ComicInfo.xml".into(), Compression::Deflate);
        writer
            .write_entry_whole(builder, xml.as_bytes())
            .await
            .map_err(|e| ThasiaError::Fatal(e.to_string()))?;

        writer.close().await.map_err(|e| ThasiaError::Fatal(e.to_string()))?;
        Ok(())
    }
}
