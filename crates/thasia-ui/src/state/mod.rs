use crate::models::{DiscoverySettings, ScanGroups};
use crate::util::paths::AppPaths;
use std::sync::{Arc, RwLock, atomic::AtomicBool};
use thasia_source::{
    LocalSource,
    suwayomi::{SuwayomiClient, SuwayomiInstaller, SuwayomiManager},
};
use tokio::sync::RwLock as AsyncRwLock;

#[derive(Default)]
pub struct ConvState {
    pub scan_result: Option<ScanGroups>,
    pub source: Option<Arc<LocalSource>>,
    pub cancel: Arc<AtomicBool>,
}

pub type SharedConvState = Arc<RwLock<ConvState>>;

pub struct DiscoveryState {
    pub paths: AppPaths,
    pub settings: Arc<AsyncRwLock<DiscoverySettings>>,
    pub installer: Arc<SuwayomiInstaller>,
    pub manager: Arc<SuwayomiManager>,
    pub client: Arc<AsyncRwLock<Option<Arc<SuwayomiClient>>>>,
}

pub type SharedDiscoveryState = Arc<DiscoveryState>;
