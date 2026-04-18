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

    // Group by volume number (default to volume 1 if none detected)
    let mut volume_map: BTreeMap<u32, Vec<_>> = BTreeMap::new();
    for parsed in parsed_images {
        let vol = parsed.identifier.volume.unwrap_or(1).max(1);
        volume_map.entry(vol).or_default().push(parsed);
    }

    // Sort pages within each volume by page number
    for pages in volume_map.values_mut() {
        pages.sort_by_key(|p| p.page_number);
    }

    // Build VolumeMeta for the frontend — only paths encoded as URLs, no bytes
    let result: Vec<VolumeMeta> = volume_map
        .iter()
        .map(|(vol_num, pages)| VolumeMeta {
            volume_num: *vol_num,
            pages: pages
                .iter()
                .enumerate()
                .map(|(page_index, p)| PageMeta {
                    volume_num: *vol_num,
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
                .collect(),
        })
        .collect();

    // Store scan result + keep source alive (holds TempDir for ZIP extractions)
    {
        let scan_data: Vec<(u32, Vec<_>)> = volume_map.into_iter().collect();
        let mut s = state.write().map_err(|e| e.to_string())?;
        s.scan_result = Some(scan_data);
        s.source = Some(std::sync::Arc::new(source));
    }

    Ok(result)
}
