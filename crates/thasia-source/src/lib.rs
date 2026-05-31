pub mod local;
pub mod prelude;
pub mod suwayomi;

pub use local::LocalSource;

use async_trait::async_trait;
use memmap2::Mmap;
use thasia_core::{Result, models::DiscoveredImage};
use tokio::sync::mpsc;

pub enum FetchedImage {
    Vec(Vec<u8>),
    Mmap(Mmap),
}

impl FetchedImage {
    pub fn len(&self) -> usize {
        self.as_ref().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }

    pub fn into_vec(self) -> Vec<u8> {
        match self {
            Self::Vec(bytes) => bytes,
            Self::Mmap(map) => map[..].to_vec(),
        }
    }
}

impl AsRef<[u8]> for FetchedImage {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Vec(bytes) => bytes,
            Self::Mmap(map) => map,
        }
    }
}

#[async_trait]
pub trait Source: Send + Sync {
    /// Recursively discovers image files and streams them into an async channel.
    async fn discover(&self) -> Result<mpsc::Receiver<DiscoveredImage>>;

    /// Reads the raw bytes of a discovered image.
    async fn fetch(&self, img: &DiscoveredImage) -> Result<FetchedImage>;

    /// Suggested maximum in-flight `fetch` calls. Local FS reads don't benefit
    /// from more than ~4-8 concurrency; network/archive sources may want more.
    /// Override per impl; default tuned for local disks.
    fn fetch_concurrency_hint(&self) -> usize {
        8
    }
}
