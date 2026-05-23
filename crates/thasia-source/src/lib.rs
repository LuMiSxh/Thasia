pub mod gallerydl;
pub mod local;

pub use local::LocalSource;

use async_trait::async_trait;
use thasia_core::{Result, models::DiscoveredImage};
use tokio::sync::mpsc;

#[async_trait]
pub trait Source: Send + Sync {
    /// Recursively discovers image files and streams them into an async channel.
    async fn discover(&self) -> Result<mpsc::Receiver<DiscoveredImage>>;

    /// Reads the raw bytes of a discovered image.
    async fn fetch(&self, img: &DiscoveredImage) -> Result<Vec<u8>>;

    /// Suggested maximum in-flight `fetch` calls. Local FS reads don't benefit
    /// from more than ~4-8 concurrency; network/archive sources may want more.
    /// Override per impl; default tuned for local disks.
    fn fetch_concurrency_hint(&self) -> usize {
        8
    }
}
