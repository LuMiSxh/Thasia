use crate::app_error::CommandResult;
use crate::conversion::run_tauri_conversion;
use crate::state::{ConvState, ConvertOptions, PipelinePlan, VolumeEdit};
use std::sync::RwLock;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub fn build_pipeline_plan(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
) -> CommandResult<PipelinePlan> {
    Ok(crate::pipeline_plan::build(options, edits))
}

#[tauri::command]
#[specta::specta]
pub async fn convert(
    options: ConvertOptions,
    edits: Vec<VolumeEdit>,
    state: State<'_, RwLock<ConvState>>,
    app: AppHandle,
) -> CommandResult<()> {
    run_tauri_conversion(options, edits, state, app).await?;
    Ok(())
}
