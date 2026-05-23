//! Shared conversion-plan types and helpers used by both the CLI and the
//! Tauri backend. Owns the bundle-mode definition and the volume-naming logic
//! so the two binaries can't drift.

use crate::models::ParsedImage;
use std::collections::BTreeMap;

/// How chapters/scan-volumes are grouped into output volumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[cfg_attr(
    feature = "tauri",
    derive(serde::Serialize, serde::Deserialize, specta::Type)
)]
#[cfg_attr(feature = "tauri", serde(rename_all = "snake_case"))]
pub enum BundleMode {
    /// Group pages by detected volume number.
    #[default]
    Auto,
    /// Merge everything into a single output volume.
    Flatten,
}

/// Options that drive how output volume filenames are constructed.
#[derive(Debug, Clone)]
pub struct VolumeNaming<'a> {
    pub name: &'a str,
    pub separator: &'a str,
    pub hide_single_volume: bool,
}

/// One output volume: identifier, final filename, and the pages it contains.
#[derive(Debug)]
pub struct VolumePlan {
    pub volume_num: u32,
    pub display_name: String,
    pub pages: Vec<ParsedImage>,
}

/// Construct the user-facing filename for an output volume.
pub fn build_volume_name(naming: &VolumeNaming, vol_num: u32, total_volumes: usize) -> String {
    if total_volumes <= 1 && naming.hide_single_volume {
        naming.name.to_string()
    } else {
        format!("{}{}{}", naming.name, naming.separator, vol_num)
    }
}

/// Auto-group parsed images by their detected volume (or merge to a single
/// volume when `bundle == Flatten`). Used by the CLI; the Tauri path builds
/// groups from frontend-supplied `VolumeEdit`s instead.
pub fn auto_group(pages: Vec<ParsedImage>, bundle: BundleMode) -> Vec<(u32, Vec<ParsedImage>)> {
    if bundle == BundleMode::Flatten {
        return if pages.is_empty() {
            Vec::new()
        } else {
            vec![(1, pages)]
        };
    }
    let mut by_vol: BTreeMap<u32, Vec<ParsedImage>> = BTreeMap::new();
    for p in pages {
        let vol = p.identifier.volume.unwrap_or(1).max(1);
        by_vol.entry(vol).or_default().push(p);
    }
    by_vol.into_iter().collect()
}

/// Wrap each (volume_num, pages) group with its display name.
pub fn apply_naming(
    groups: Vec<(u32, Vec<ParsedImage>)>,
    naming: &VolumeNaming,
) -> Vec<VolumePlan> {
    let total = groups.len();
    groups
        .into_iter()
        .map(|(vol_num, pages)| VolumePlan {
            volume_num: vol_num,
            display_name: build_volume_name(naming, vol_num, total),
            pages,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChapterIdentifier, DiscoveredImage, ParsedImage};
    use std::path::PathBuf;

    fn page(vol: Option<u32>, page: u32) -> ParsedImage {
        ParsedImage {
            source: DiscoveredImage {
                absolute_path: PathBuf::from("/x"),
                relative_path: "x".into(),
            },
            identifier: ChapterIdentifier {
                volume: vol,
                chapter: None,
            },
            page_number: page as f32,
            is_cover: false,
        }
    }

    #[test]
    fn build_volume_name_hides_when_single_and_flag_set() {
        let n = VolumeNaming { name: "Manga", separator: " - ", hide_single_volume: true };
        assert_eq!(build_volume_name(&n, 1, 1), "Manga");
    }

    #[test]
    fn build_volume_name_shows_number_for_multi() {
        let n = VolumeNaming { name: "Manga", separator: " - ", hide_single_volume: true };
        assert_eq!(build_volume_name(&n, 3, 5), "Manga - 3");
    }

    #[test]
    fn build_volume_name_shows_number_when_flag_unset() {
        let n = VolumeNaming { name: "Manga", separator: " - ", hide_single_volume: false };
        assert_eq!(build_volume_name(&n, 1, 1), "Manga - 1");
    }

    #[test]
    fn auto_group_buckets_by_volume() {
        let pages = vec![page(Some(2), 0), page(Some(1), 0), page(Some(2), 1)];
        let groups = auto_group(pages, BundleMode::Auto);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, 1);
        assert_eq!(groups[1].0, 2);
        assert_eq!(groups[1].1.len(), 2);
    }

    #[test]
    fn auto_group_flatten_collapses_to_one() {
        let pages = vec![page(Some(2), 0), page(Some(1), 0), page(Some(3), 1)];
        let groups = auto_group(pages, BundleMode::Flatten);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, 1);
        assert_eq!(groups[0].1.len(), 3);
    }

    #[test]
    fn auto_group_treats_volume_zero_as_one() {
        let pages = vec![page(Some(0), 0), page(None, 1)];
        let groups = auto_group(pages, BundleMode::Auto);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, 1);
    }
}
