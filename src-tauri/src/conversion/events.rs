use crate::events::{
    ConversionCompleteEvent, ImageProgressEvent, VolumeCompleteEvent, VolumeStartEvent,
};
use tauri::AppHandle;
use tauri_specta::Event;

pub(crate) trait ConversionEvents: Send + Sync {
    fn volume_started(&self, event: VolumeStartEvent);
    fn image_progress(&self, event: ImageProgressEvent);
    fn volume_completed(&self, event: VolumeCompleteEvent);
    fn conversion_completed(&self, event: ConversionCompleteEvent);
}

pub(crate) struct TauriConversionEvents<'a> {
    app: &'a AppHandle,
}

impl<'a> TauriConversionEvents<'a> {
    pub(crate) fn new(app: &'a AppHandle) -> Self {
        Self { app }
    }
}

impl ConversionEvents for TauriConversionEvents<'_> {
    fn volume_started(&self, event: VolumeStartEvent) {
        event.emit(self.app).ok();
    }

    fn image_progress(&self, event: ImageProgressEvent) {
        event.emit(self.app).ok();
    }

    fn volume_completed(&self, event: VolumeCompleteEvent) {
        event.emit(self.app).ok();
    }

    fn conversion_completed(&self, event: ConversionCompleteEvent) {
        event.emit(self.app).ok();
    }
}
