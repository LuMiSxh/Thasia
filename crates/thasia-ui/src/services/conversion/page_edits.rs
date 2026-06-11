use crate::{
    error::{AppError, AppResult},
    models::{PageEditSource, VolumeEdit},
};
use std::collections::BTreeMap;
use std::path::Path;
use thasia_core::prelude::{ChapterIdentifier, DiscoveredImage, ParsedImage};
use thasia_source::LocalSource;

const MAX_CUSTOM_IMAGE_BYTES: u64 = 512 * 1024 * 1024;

/// Materialize the final ordered page list for one VolumeEdit by looking each
/// PageEditEntry up in the scan map or constructing a custom-path entry.
pub(crate) fn resolve_edit_pages(
    edit: &VolumeEdit,
    scan_map: &BTreeMap<u32, Vec<ParsedImage>>,
) -> AppResult<Vec<ParsedImage>> {
    let mut final_pages = Vec::new();

    for entry in &edit.pages {
        if entry.excluded {
            continue;
        }

        match &entry.source {
            PageEditSource::Original {
                page_index,
                source_volume_num,
            } => {
                let src_vol = source_volume_num.unwrap_or(edit.volume_num);
                if let Some(page) = scan_map
                    .get(&src_vol)
                    .and_then(|v| v.get(*page_index as usize))
                {
                    final_pages.push(page.clone());
                }
            }
            PageEditSource::Custom { path } => {
                final_pages.push(custom_page(edit.volume_num, final_pages.len(), path)?);
            }
        }
    }

    for (i, page) in final_pages.iter_mut().enumerate() {
        page.page_number = i as f32;
    }
    Ok(final_pages)
}

fn custom_page(volume_num: u32, page_index: usize, path: &Path) -> AppResult<ParsedImage> {
    validate_custom_image_path(path)?;
    let relative_path = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    Ok(ParsedImage {
        source: DiscoveredImage {
            absolute_path: path.to_path_buf(),
            relative_path,
        },
        identifier: ChapterIdentifier {
            volume: Some(volume_num),
            chapter: None,
        },
        page_number: page_index as f32,
        is_cover: page_index == 0,
    })
}

fn validate_custom_image_path(path: &Path) -> AppResult<()> {
    if !LocalSource::is_supported_image_path(path) {
        return Err(AppError::UnsupportedCustomImage {
            path: path.to_path_buf(),
        });
    }
    let metadata = std::fs::metadata(path).map_err(|source| AppError::CustomImageMetadata {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(AppError::CustomImageNotFile {
            path: path.to_path_buf(),
        });
    }
    if metadata.len() > MAX_CUSTOM_IMAGE_BYTES {
        return Err(AppError::CustomImageTooLarge {
            path: path.to_path_buf(),
            bytes: metadata.len(),
        });
    }
    Ok(())
}
