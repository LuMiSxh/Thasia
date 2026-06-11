use crate::{
    error::AppResult,
    models::{PageMeta, ScanGroups, VolumeMeta},
    state::SharedConvState,
};
use ordered_float::OrderedFloat;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use thasia_parser::{Resolver, RuleConfig};
use thasia_source::{LocalSource, Source};

pub async fn scan_source(path: PathBuf, state: SharedConvState) -> AppResult<Vec<VolumeMeta>> {
    let source = if LocalSource::is_archive(&path) {
        LocalSource::from_archive(path).await?
    } else {
        LocalSource::new(path)
    };
    let (volumes, groups) = scan_local_source(&source).await?;
    let mut state = state.write().map_err(|error| error.to_string())?;
    state.scan_result = Some(groups);
    state.source = Some(Arc::new(source));
    Ok(volumes)
}

async fn scan_local_source(source: &LocalSource) -> AppResult<(Vec<VolumeMeta>, ScanGroups)> {
    let resolver = Resolver::new(RuleConfig::default());
    let mut rx = source.discover().await?;
    let mut discovered = Vec::new();
    while let Some(image) = rx.recv().await {
        discovered.push(image);
    }
    let mut chapters: BTreeMap<(u32, OrderedFloat<f32>), Vec<_>> = BTreeMap::new();
    for parsed in resolver.resolve_batch(discovered) {
        let volume = parsed.identifier.volume.unwrap_or(1).max(1);
        let chapter = OrderedFloat(parsed.identifier.chapter.unwrap_or(0.0));
        chapters.entry((volume, chapter)).or_default().push(parsed);
    }
    for pages in chapters.values_mut() {
        pages.sort_by(|a, b| {
            b.is_cover
                .cmp(&a.is_cover)
                .then_with(|| a.page_number.total_cmp(&b.page_number))
        });
    }

    let mut volumes = Vec::new();
    let mut groups = Vec::new();
    for ((source_volume_num, _), pages) in chapters {
        let volume_num = volumes.len() as u32 + 1;
        let metadata = pages
            .iter()
            .enumerate()
            .map(|(page_index, page)| PageMeta {
                volume_num,
                page_index: page_index as u32,
                path: page.source.absolute_path.clone(),
                file_name: page
                    .source
                    .absolute_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_string(),
            })
            .collect();
        volumes.push(VolumeMeta {
            volume_num,
            source_volume_num,
            pages: metadata,
        });
        groups.push((volume_num, pages));
    }
    Ok((volumes, groups))
}
