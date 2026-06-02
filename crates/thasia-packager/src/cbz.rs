use crate::{Generator, PackagerError, Result};
use async_trait::async_trait;
use async_zip::{
    Compression, ZipEntryBuilder, base::write::ZipFileWriter as BaseZipFileWriter,
    tokio::write::ZipFileWriter,
};
use std::path::{Path, PathBuf};
use thasia_core::{escape_xml_text, models::ProcessedImage, sanitize_filename_component};
use tokio::fs::File;

const COMIC_INFO: &str = include_str!("../templates/comic_info.xml");

pub struct CbzGenerator {
    writer: Option<ZipFileWriter<File>>,
    output_path: Option<PathBuf>,
    volume_name: String,
    page_count: u32,
}

impl CbzGenerator {
    pub fn new() -> Self {
        Self {
            writer: None,
            output_path: None,
            volume_name: String::new(),
            page_count: 0,
        }
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
        tokio::fs::create_dir_all(output_dir).await?;
        let safe_volume_name = sanitize_filename_component(volume_name)?;
        let path = output_dir.join(format!("{safe_volume_name}.cbz"));
        let file = File::create(&path).await?;
        self.writer = Some(BaseZipFileWriter::with_tokio(file));
        self.output_path = Some(path);
        self.volume_name = safe_volume_name;
        Ok(())
    }

    async fn add_page(&mut self, img: ProcessedImage) -> Result<()> {
        let writer = self
            .writer
            .as_mut()
            .ok_or(PackagerError::NotInitialized("CBZ writer"))?;

        // page_number is f32 (parser/sort key) but the caller reassigns it to
        // sequential integers before this point, so cast back for filename formatting.
        let filename = format!(
            "page_{:04}.{}",
            (img.parsed_data.page_number as u32) + 1,
            img.ext
        );
        // AVIF and WebP are already compressed — use Stored to avoid double compression.
        let compression = match img.ext.as_str() {
            "avif" | "webp" => Compression::Stored,
            _ => Compression::Deflate,
        };

        let builder = ZipEntryBuilder::new(filename.into(), compression);
        writer.write_entry_whole(builder, &img.image_data).await?;

        self.page_count += 1;
        Ok(())
    }

    fn output_path(&self) -> Option<PathBuf> {
        self.output_path.clone()
    }

    async fn finalize(mut self: Box<Self>) -> Result<()> {
        let mut writer = self
            .writer
            .take()
            .ok_or(PackagerError::NotInitialized("CBZ writer"))?;

        let title = escape_xml_text(&self.volume_name);
        let xml = COMIC_INFO
            .replace("%title%", &title)
            .replace("%pagecount%", &self.page_count.to_string());

        let builder = ZipEntryBuilder::new("ComicInfo.xml".into(), Compression::Deflate);
        writer.write_entry_whole(builder, xml.as_bytes()).await?;

        writer.close().await?;
        Ok(())
    }
}
