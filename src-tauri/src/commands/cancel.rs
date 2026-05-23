use crate::state::ConvState;
use std::sync::RwLock;
use std::sync::atomic::Ordering;
use tauri::State;

/// Request cooperative cancellation of an in-flight `convert`.
///
/// Cancellation is checked between volumes and between encoded-image deliveries
/// inside a volume, so the request takes effect within ~one image's encode time
/// in the worst case (not mid-encode).
#[tauri::command]
#[specta::specta]
pub async fn cancel_conversion(state: State<'_, RwLock<ConvState>>) -> Result<(), String> {
    let s = state.read().map_err(|e| e.to_string())?;
    s.cancel.store(true, Ordering::SeqCst);
    Ok(())
}
