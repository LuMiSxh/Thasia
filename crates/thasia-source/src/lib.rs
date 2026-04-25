pub mod gallerydl;
pub mod local;

pub use local::LocalSource;

use async_trait::async_trait;
use thasia_core::{Result, models::DiscoveredImage};
use tokio::sync::mpsc;

#[async_trait]
pub trait Source {
    /// Recursively discovers image files and streams them into an async channel.
    async fn discover(&self) -> Result<mpsc::Receiver<DiscoveredImage>>;

    /// Reads the raw bytes of a discovered image.
    async fn fetch(&self, img: &DiscoveredImage) -> Result<Vec<u8>>;

    /// Cleans up temp directories (e.g., extracted ZIPs).
    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
