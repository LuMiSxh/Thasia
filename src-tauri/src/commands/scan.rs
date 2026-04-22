use crate::protocol::image_url;
use crate::state::{ConvState, PageMeta, VolumeMeta};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::RwLock;
use tauri::State;
use thasia_parser::{Resolver, RuleConfig};
use thasia_source::LocalSource;

#[tauri::command]
#[specta::specta]
pub async fn scan_source(
    path: String,
    state: State<'_, RwLock<ConvState>>,
) -> Result<Vec<VolumeMeta>, String> {
    let source_path = PathBuf::from(&path);

    // Build the source — handles both directories and ZIP/CBZ archives
    let source = if LocalSource::is_archive(&source_path) {
        LocalSource::from_archive(source_path).await.map_err(|e| e.to_string())?
    } else {
        LocalSource::new(source_path)
    };

    let resolver = Resolver::new(RuleConfig::default());

    // Discover all images
    let mut rx = {
        use thasia_source::Source;
        source.discover().await.map_err(|e| e.to_string())?
    };

    // Parse each discovered image
    let mut parsed_images = Vec::new();
    while let Some(img) = rx.recv().await {
        match resolver.resolve(img) {
            Ok(parsed) => parsed_images.push(parsed),
            Err(e) => tracing::warn!("Skipping unresolved image: {e}"),
        }
    }

    // Group by (volume, chapter) to preserve per-chapter granularity.
    // chapter f32 is converted to bits for BTreeMap ordering — safe for
    // non-negative chapter numbers (which the parser always produces).
    let mut chapter_map: BTreeMap<(u32, u32), Vec<_>> = BTreeMap::new();
    for parsed in parsed_images {
        let vol = parsed.identifier.volume.unwrap_or(1).max(1);
        let ch_bits = parsed.identifier.chapter.map(f32::to_bits).unwrap_or(0);
        chapter_map.entry((vol, ch_bits)).or_default().push(parsed);
    }

    // Sort pages within each chapter group by page number
    for pages in chapter_map.values_mut() {
        pages.sort_by_key(|p| p.page_number);
    }

    // Assign sequential scan indices (1, 2, …) as unique lookup keys.
    // source_volume_num carries the actual parsed volume for frontend grouping.
    let mut result: Vec<VolumeMeta> = Vec::new();
    let mut scan_result_for_state: Vec<(u32, Vec<_>)> = Vec::new();

    for ((source_vol, _ch_bits), pages) in chapter_map {
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

    // Store scan result + keep source alive (holds TempDir for ZIP extractions)
    {
        let mut s = state.write().map_err(|e| e.to_string())?;
        s.scan_result = Some(scan_result_for_state);
        s.source = Some(std::sync::Arc::new(source));
    }

    Ok(result)
}
