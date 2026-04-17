use crate::Generator;
use async_trait::async_trait;
use async_zip::{base::write::ZipFileWriter as BaseZipFileWriter, tokio::write::ZipFileWriter, Compression, ZipEntryBuilder};
use std::path::Path;
use thasia_core::{models::ProcessedImage, Result, ThasiaError};
use tokio::fs::File;

const COMIC_INFO: &str = include_str!("../templates/comic_info.xml");

pub struct CbzGenerator {
    writer: Option<ZipFileWriter<File>>,
    volume_name: String,
    page_count: u32,
}

impl CbzGenerator {
    pub fn new() -> Self {
        Self { writer: None, volume_name: String::new(), page_count: 0 }
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
        self.volume_name = volume_name.to_string();
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

        let xml = COMIC_INFO
            .replace("%title%", &self.volume_name)
            .replace("%pagecount%", &self.page_count.to_string());

        let builder = ZipEntryBuilder::new("ComicInfo.xml".into(), Compression::Deflate);
        writer
            .write_entry_whole(builder, xml.as_bytes())
            .await
            .map_err(|e| ThasiaError::Fatal(e.to_string()))?;

        writer.close().await.map_err(|e| ThasiaError::Fatal(e.to_string()))?;
        Ok(())
    }
}
