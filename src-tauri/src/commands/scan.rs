use crate::protocol::image_url;
use crate::state::{ConvState, PageMeta, ScanGroups, VolumeMeta};
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::RwLock;
use tauri::State;
use thasia_parser::{Resolver, RuleConfig};
use thasia_source::{LocalSource, Source};

#[tauri::command]
#[specta::specta]
pub async fn scan_source(
    path: String,
    state: State<'_, RwLock<ConvState>>,
) -> Result<Vec<VolumeMeta>, String> {
    let source_path = PathBuf::from(&path);

    // Build the source — handles both directories and ZIP/CBZ archives
    let source = if LocalSource::is_archive(&source_path) {
        LocalSource::from_archive(source_path)
            .await
            .map_err(|e| e.to_string())?
    } else {
        LocalSource::new(source_path)
    };

    let (result, scan_result_for_state) = scan_local_source(&source).await?;

    // Store scan result + keep source alive (holds TempDir for ZIP extractions)
    {
        let mut s = state.write().map_err(|e| e.to_string())?;
        s.scan_result = Some(scan_result_for_state);
        s.source = Some(std::sync::Arc::new(source));
    }

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn scan_current_source(
    state: State<'_, RwLock<ConvState>>,
) -> Result<Vec<VolumeMeta>, String> {
    let source = {
        let s = state.read().map_err(|e| e.to_string())?;
        s.source
            .clone()
            .ok_or_else(|| "No source is prepared for conversion.".to_string())?
    };

    let (result, scan_result_for_state) = scan_local_source(source.as_ref()).await?;

    {
        let mut s = state.write().map_err(|e| e.to_string())?;
        s.scan_result = Some(scan_result_for_state);
    }

    Ok(result)
}

async fn scan_local_source(source: &LocalSource) -> Result<(Vec<VolumeMeta>, ScanGroups), String> {
    let resolver = Resolver::new(RuleConfig::default());

    // Discover all images
    let mut rx = source.discover().await.map_err(|e| e.to_string())?;

    // Collect discovered images for batch resolution (so natord-fallback can
    // run across siblings for directories with unparseable filenames).
    let mut discovered = Vec::new();
    while let Some(img) = rx.recv().await {
        discovered.push(img);
    }
    let parsed_images = resolver.resolve_batch(discovered);

    // Group by (volume, chapter) to preserve per-chapter granularity.
    // OrderedFloat gives a real total order over f32 (NaN sorts after everything).
    let mut chapter_map: BTreeMap<(u32, OrderedFloat<f32>), Vec<_>> = BTreeMap::new();
    for parsed in parsed_images {
        let vol = parsed.identifier.volume.unwrap_or(1).max(1);
        let chapter = OrderedFloat(parsed.identifier.chapter.unwrap_or(0.0));
        chapter_map.entry((vol, chapter)).or_default().push(parsed);
    }

    // Sort pages within each chapter group: covers first, then by page_number.
    for pages in chapter_map.values_mut() {
        pages.sort_by(|a, b| {
            b.is_cover
                .cmp(&a.is_cover)
                .then_with(|| a.page_number.total_cmp(&b.page_number))
        });
    }

    // Assign sequential scan indices (1, 2, …) as unique lookup keys.
    // source_volume_num carries the actual parsed volume for frontend grouping.
    let mut result: Vec<VolumeMeta> = Vec::new();
    let mut scan_result_for_state: Vec<(u32, Vec<_>)> = Vec::new();

    for ((source_vol, _chapter), pages) in chapter_map {
        let scan_num = (result.len() as u32) + 1;

        let page_metas: Vec<PageMeta> = pages
            .iter()
            .enumerate()
            .map(|(page_index, p)| PageMeta {
                volume_num: scan_num,
                page_index: page_index as u32,
                url: image_url(&p.source.absolute_path),
                file_name: p
                    .source
                    .absolute_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string(),
            })
            .collect();

        result.push(VolumeMeta {
            volume_num: scan_num,
            source_volume_num: source_vol,
            pages: page_metas,
        });

        scan_result_for_state.push((scan_num, pages));
    }

    Ok((result, scan_result_for_state))
}
