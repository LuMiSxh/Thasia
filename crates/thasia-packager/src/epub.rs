use crate::{Generator, PackagerError, Result};
use async_trait::async_trait;
use epub_builder::{EpubBuilder, EpubContent, EpubVersion, ZipLibrary};
use std::io::{BufWriter, Cursor};
use std::path::{Path, PathBuf};
use thasia_core::{
    models::{Direction, ProcessedImage},
    sanitize_filename_component,
};

const PAGE_XHTML: &str = include_str!("../templates/epub_page.xhtml");
const PAGE_CSS: &[u8] = include_bytes!("../templates/epub_style.css");

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}

pub struct EpubGenerator {
    output_dir: PathBuf,
    volume_name: String,
    direction: Direction,
    pages: Vec<ProcessedImage>,
}

impl EpubGenerator {
    pub fn new() -> Self {
        Self {
            output_dir: PathBuf::new(),
            volume_name: String::new(),
            direction: Direction::default(),
            pages: Vec::new(),
        }
    }

    pub fn with_direction(mut self, dir: Direction) -> Self {
        self.direction = dir;
        self
    }
}

impl Default for EpubGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for EpubGenerator {
    async fn init(&mut self, output_dir: &Path, volume_name: &str) -> Result<()> {
        tokio::fs::create_dir_all(output_dir).await?;
        let safe_volume_name = sanitize_filename_component(volume_name)?;
        self.output_dir = output_dir.to_path_buf();
        self.volume_name = safe_volume_name;
        Ok(())
    }

    async fn add_page(&mut self, img: ProcessedImage) -> Result<()> {
        self.pages.push(img);
        Ok(())
    }

    fn output_path(&self) -> Option<PathBuf> {
        if self.output_dir.as_os_str().is_empty() || self.volume_name.is_empty() {
            return None;
        }
        Some(self.output_dir.join(format!("{}.epub", self.volume_name)))
    }

    async fn finalize(self: Box<Self>) -> Result<()> {
        let output_path = self.output_dir.join(format!("{}.epub", self.volume_name));
        let volume_name = self.volume_name.clone();
        let direction = self.direction;
        // Pages already arrive in order — convert.rs (Tauri) and the CLI both
        // feed add_page sequentially after sorting. No internal sort needed.
        let pages = self.pages;

        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut epub = EpubBuilder::new(ZipLibrary::new().map_err(PackagerError::epub)?)
                .map_err(PackagerError::epub)?;

            epub.epub_version(EpubVersion::V30);
            epub.metadata("rendition:layout", "pre-paginated")
                .map_err(PackagerError::epub)?;
            epub.metadata("title", &volume_name)
                .map_err(PackagerError::epub)?;

            let dir_str = match direction {
                Direction::Ltr => "ltr",
                Direction::Rtl => "rtl",
            };
            epub.metadata("direction", dir_str)
                .map_err(PackagerError::epub)?;

            epub.stylesheet(Cursor::new(PAGE_CSS))
                .map_err(PackagerError::epub)?;

            for (i, img) in pages.iter().enumerate() {
                let page_num = i + 1;
                let image_path = format!("images/page_{:04}.{}", page_num, img.ext);
                let xhtml_path = format!("page_{:04}.xhtml", page_num);
                let mime = mime_for(&img.ext);

                if img.parsed_data.is_cover {
                    epub.add_cover_image(&image_path, Cursor::new(&img.image_data[..]), mime)
                        .map_err(PackagerError::epub)?;
                } else {
                    epub.add_resource(&image_path, Cursor::new(&img.image_data[..]), mime)
                        .map_err(PackagerError::epub)?;
                }

                let xhtml = PAGE_XHTML
                    .replace("%title%", &xhtml_path)
                    .replace("%src%", &image_path)
                    .replace("%alt%", &format!("Page {}", page_num));

                epub.add_content(EpubContent::new(xhtml_path, xhtml.as_bytes()))
                    .map_err(PackagerError::epub)?;
            }

            let file = std::fs::File::create(&output_path)?;
            let buf_writer = BufWriter::with_capacity(128 * 1024, file);
            epub.generate(buf_writer).map_err(PackagerError::epub)?;

            Ok(())
        })
        .await?
    }
}
