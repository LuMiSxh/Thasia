//! Assembles classified path components into a final ParsedImage.
//!
//! Strategy:
//! 1. Strip the file extension from the filename, classify each path component.
//! 2. Filename → page (or `None` if unclassifiable; natord fallback fills it later).
//! 3. Walk directories innermost-out for VolumeMarker / ChapterMarker / HakunekoFolder.
//! 4. Disambiguate any leftover PureNumber directories using a depth-position rule
//!    (innermost = chapter, next-outer = volume) or the user-configurable
//!    depth_mapping fallback.
//! 5. After the whole batch is classified, group by parent directory; if any
//!    sibling failed step 2, all siblings in that directory get natord positions
//!    (deterministic order, no half-parsed half-fallback collisions).

use crate::{
    classifier::{Component, ComponentKind, classify},
    rules::RuleConfig,
};
use std::collections::BTreeMap;
use std::path::Path;
use thasia_core::models::{ChapterIdentifier, DiscoveredImage, ParsedImage};

pub struct Resolver {
    config: RuleConfig,
}

/// Intermediate result before natord-fallback runs.
struct PartialFields {
    volume: Option<u32>,
    chapter: Option<f32>,
    page: Option<f32>,
    is_cover: bool,
}

impl Resolver {
    pub fn new(config: RuleConfig) -> Self {
        Self { config }
    }

    /// Resolve a single image. Returns Ok even when no page number could be
    /// extracted — the batch resolver assigns a fallback. If you have many
    /// images, prefer `resolve_batch` so siblings get a consistent natord-based
    /// ordering when filenames are unparseable.
    pub fn resolve(&self, img: DiscoveredImage) -> Result<ParsedImage, String> {
        let mut fields = self.classify_path(&img.relative_path);
        // Default to 1.0 when truly nothing parseable — caller can override.
        let page = fields.page.take().unwrap_or(1.0);
        Ok(ParsedImage {
            source: img,
            identifier: ChapterIdentifier {
                volume: fields.volume,
                chapter: fields.chapter,
            },
            page_number: page,
            is_cover: fields.is_cover,
        })
    }

    /// Resolve a batch of images. For directories where any filename couldn't
    /// be classified into a page number, ALL files in that directory get natord
    /// positions — avoids the half-parsed/half-fallback ordering hazard.
    pub fn resolve_batch(&self, imgs: Vec<DiscoveredImage>) -> Vec<ParsedImage> {
        // Step 1: classify every image.
        let mut partials: Vec<(DiscoveredImage, PartialFields)> = imgs
            .into_iter()
            .map(|img| {
                let fields = self.classify_path(&img.relative_path);
                (img, fields)
            })
            .collect();

        // Step 2: group by parent directory.
        let mut by_parent: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, (img, _)) in partials.iter().enumerate() {
            let parent = Path::new(&img.relative_path)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            by_parent.entry(parent).or_default().push(i);
        }

        // Step 3: per-directory natord-fallback.
        for (_, indices) in by_parent {
            let has_unparseable = indices.iter().any(|&i| partials[i].1.page.is_none());
            if !has_unparseable {
                continue;
            }
            // Sort by natord of the filename; assign positions in that order.
            let mut sorted = indices.clone();
            sorted.sort_by(|&a, &b| {
                let fa = Path::new(&partials[a].0.relative_path)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let fb = Path::new(&partials[b].0.relative_path)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                natord::compare(&fa, &fb)
            });
            // Use 1-based positions; covers naturally come first because their
            // stems usually sort early (`cover.jpg` < `001.jpg` natord-wise).
            // We also pin is_cover entries to position 0 explicitly below.
            for (pos, &idx) in sorted.iter().enumerate() {
                let is_cover = partials[idx].1.is_cover;
                partials[idx].1.page = Some(if is_cover { 0.0 } else { (pos + 1) as f32 });
            }
        }

        // Step 4: materialize ParsedImage.
        partials
            .into_iter()
            .map(|(img, fields)| ParsedImage {
                source: img,
                identifier: ChapterIdentifier {
                    volume: fields.volume,
                    chapter: fields.chapter,
                },
                page_number: fields.page.unwrap_or(1.0),
                is_cover: fields.is_cover,
            })
            .collect()
    }

    /// Classify all components of a relative path and assemble PartialFields.
    fn classify_path(&self, relative_path: &str) -> PartialFields {
        let path = Path::new(relative_path);
        let components: Vec<&str> = path.iter().filter_map(|s| s.to_str()).collect();
        if components.is_empty() {
            return PartialFields {
                volume: None,
                chapter: None,
                page: None,
                is_cover: false,
            };
        }

        let (filename_full, dirs) = components.split_last().unwrap();
        // Strip extension from the filename for classification.
        let filename_stem = Path::new(filename_full)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename_full);

        let mut fields = PartialFields {
            volume: None,
            chapter: None,
            page: None,
            is_cover: false,
        };

        // Filename → page (or cover).
        match classify(filename_stem, ComponentKind::Filename) {
            Component::Cover => fields.is_cover = true,
            Component::PageMarker(n) | Component::Spread(n) | Component::PureNumber(n) => {
                fields.page = Some(n);
            }
            Component::VolumeMarker(_) | Component::ChapterMarker(_) | Component::HakunekoFolder(..) => {
                // Filename shaped like a directory marker — rare; leave page unset.
            }
            Component::Noise => {
                // page stays None → natord-fallback in resolve_batch.
            }
        }

        // Walk directories innermost-out for volume/chapter.
        let mut pure_numbers: Vec<(usize, f32)> = Vec::new();
        for (depth, dir) in dirs.iter().enumerate() {
            match classify(dir, ComponentKind::Directory) {
                Component::VolumeMarker(v) => {
                    if fields.volume.is_none() {
                        fields.volume = Some(v);
                    }
                }
                Component::ChapterMarker(c) => {
                    if fields.chapter.is_none() {
                        fields.chapter = Some(c);
                    }
                }
                Component::HakunekoFolder(v, c) => {
                    if fields.volume.is_none() {
                        fields.volume = Some(v);
                    }
                    if fields.chapter.is_none() {
                        fields.chapter = Some(c);
                    }
                }
                Component::PureNumber(n) => {
                    pure_numbers.push((depth, n));
                }
                _ => {}
            }
        }

        // Disambiguate leftover PureNumber directories. Innermost = chapter,
        // next-outer = volume (typical convention). Only fills slots still None.
        // Iterate innermost-first.
        if !pure_numbers.is_empty() {
            pure_numbers.sort_by(|a, b| b.0.cmp(&a.0));
            let mut iter = pure_numbers.iter();
            if fields.chapter.is_none()
                && let Some((_, n)) = iter.next()
            {
                fields.chapter = Some(*n);
            }
            if fields.volume.is_none()
                && let Some((_, n)) = iter.next()
            {
                fields.volume = Some(*n as u32);
            }
        }

        // User-configurable depth_mapping fallback (preserves the old behaviour
        // for anyone who set up explicit mappings). Only fires for PureNumber
        // components — Noise dirs (e.g. "My Manga - 5") must not contribute.
        if self.config.enable_depth_fallback {
            for (depth, dir) in dirs.iter().enumerate() {
                if let Some(level) = self.config.depth_mapping.get(depth)
                    && let Component::PureNumber(n) = classify(dir, ComponentKind::Directory)
                {
                    match level.as_str() {
                        "volume" if fields.volume.is_none() => {
                            fields.volume = Some(n as u32);
                        }
                        "chapter" if fields.chapter.is_none() => {
                            fields.chapter = Some(n);
                        }
                        "page" if fields.page.is_none() => {
                            fields.page = Some(n);
                        }
                        _ => {}
                    }
                }
            }
        }

        fields
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn img(rel: &str) -> DiscoveredImage {
        DiscoveredImage {
            absolute_path: PathBuf::from("/root").join(rel),
            relative_path: rel.to_string(),
        }
    }

    fn imgs(paths: &[&str]) -> Vec<DiscoveredImage> {
        paths.iter().map(|p| img(p)).collect()
    }

    fn resolver() -> Resolver {
        Resolver::new(RuleConfig::default())
    }

    /// Helper: sort parsed images by (volume, chapter, is_cover desc, page) so
    /// test assertions are stable regardless of insertion order.
    fn ordered(mut v: Vec<ParsedImage>) -> Vec<ParsedImage> {
        v.sort_by(|a, b| {
            let av = a.identifier.volume.unwrap_or(1);
            let bv = b.identifier.volume.unwrap_or(1);
            av.cmp(&bv)
                .then_with(|| {
                    let ac = a.identifier.chapter.unwrap_or(0.0);
                    let bc = b.identifier.chapter.unwrap_or(0.0);
                    ac.total_cmp(&bc)
                })
                .then_with(|| b.is_cover.cmp(&a.is_cover))
                .then_with(|| a.page_number.total_cmp(&b.page_number))
        });
        v
    }

    // ───── existing schemas the old parser handled correctly ─────────────────

    #[test]
    fn hakuneko_style_dir() {
        let p = resolver().resolve(img("001-002/001.jpg")).unwrap();
        assert_eq!(p.identifier.volume, Some(1));
        assert_eq!(p.identifier.chapter, Some(2.0));
        assert_eq!(p.page_number, 1.0);
        assert!(!p.is_cover);
    }

    #[test]
    fn explicit_vol_ch_labels() {
        let p = resolver().resolve(img("Vol 1/Ch 04/001.jpg")).unwrap();
        assert_eq!(p.identifier.volume, Some(1));
        assert_eq!(p.identifier.chapter, Some(4.0));
        assert_eq!(p.page_number, 1.0);
    }

    #[test]
    fn float_chapter() {
        let p = resolver().resolve(img("Vol 2/Ch 10.5/001.png")).unwrap();
        assert_eq!(p.identifier.chapter, Some(10.5));
    }

    #[test]
    fn pure_number_depth_fallback() {
        // "1/14/001.jpg" — depth_mapping kicks in (volume/chapter/page).
        let p = resolver().resolve(img("1/14/001.jpg")).unwrap();
        assert_eq!(p.identifier.volume, Some(1));
        assert_eq!(p.identifier.chapter, Some(14.0));
        assert_eq!(p.page_number, 1.0);
    }

    #[test]
    fn cover_detection_in_dir() {
        let p = resolver().resolve(img("Vol 1/cover.jpg")).unwrap();
        assert!(p.is_cover);
    }

    // ───── the cases the old parser got WRONG ────────────────────────────────

    #[test]
    fn series_title_doesnt_leak_volume_token() {
        // The old parser saw `V` in "Verse" and set volume=1. New parser
        // classifies "Spider-Verse" as Noise — no spurious volume.
        let p = resolver().resolve(img("Spider-Verse/001.jpg")).unwrap();
        assert_eq!(p.identifier.volume, None);
        assert_eq!(p.page_number, 1.0);
    }

    #[test]
    fn title_with_dash_number_isnt_hakuneko() {
        // "My Manga - 5" is title noise, not Hakuneko (\d-\d).
        let p = resolver().resolve(img("My Manga - 5/001.jpg")).unwrap();
        assert_eq!(p.identifier.volume, None);
        assert_eq!(p.identifier.chapter, None);
        assert_eq!(p.page_number, 1.0);
    }

    // ───── double-page spreads (archive case) ────────────────────────────────

    #[test]
    fn spread_filenames_order_correctly() {
        let parsed = resolver().resolve_batch(imgs(&[
            "001_002.jpg",
            "003.jpg",
            "004_005.jpg",
            "006.jpg",
        ]));
        let pages: Vec<f32> = ordered(parsed).into_iter().map(|p| p.page_number).collect();
        assert_eq!(pages, vec![1.0, 3.0, 4.0, 6.0]);
    }

    // ───── sub-page inserts: p35, p35.5, p35.6, p36 ──────────────────────────

    #[test]
    fn sub_pages_order_naturally() {
        let parsed = resolver().resolve_batch(imgs(&[
            "p36.jpg",
            "p35.5.jpg",
            "p35.jpg",
            "p35.6.jpg",
        ]));
        let pages: Vec<f32> = ordered(parsed).into_iter().map(|p| p.page_number).collect();
        assert_eq!(pages, vec![35.0, 35.5, 35.6, 36.0]);
    }

    #[test]
    fn sub_pages_pure_numeric() {
        // No "p" prefix — pure numeric with decimal.
        let parsed = resolver().resolve_batch(imgs(&["36.jpg", "35.5.jpg", "35.jpg", "35.6.jpg"]));
        let pages: Vec<f32> = ordered(parsed).into_iter().map(|p| p.page_number).collect();
        assert_eq!(pages, vec![35.0, 35.5, 35.6, 36.0]);
    }

    // ───── flat archive (the common CBZ extraction case) ─────────────────────

    #[test]
    fn flat_archive_pure_numeric_orders_by_page() {
        let parsed = resolver().resolve_batch(imgs(&[
            "003.jpg", "001.jpg", "002.jpg", "010.jpg", "005.jpg",
        ]));
        let pages: Vec<f32> = ordered(parsed).into_iter().map(|p| p.page_number).collect();
        assert_eq!(pages, vec![1.0, 2.0, 3.0, 5.0, 10.0]);
    }

    #[test]
    fn flat_archive_cover_plus_pages() {
        let parsed =
            resolver().resolve_batch(imgs(&["cover.jpg", "001.jpg", "002.jpg", "003.jpg"]));
        let parsed = ordered(parsed);
        assert!(parsed[0].is_cover);
        let pages: Vec<f32> = parsed.iter().skip(1).map(|p| p.page_number).collect();
        assert_eq!(pages, vec![1.0, 2.0, 3.0]);
    }

    // ───── unparseable filenames fall back to natord ─────────────────────────

    #[test]
    fn unparseable_filenames_use_natord_position() {
        // All Noise — no number anywhere. natord-fallback should assign 1,2,3,4.
        let parsed = resolver().resolve_batch(imgs(&[
            "prologue.jpg",
            "intro.jpg",
            "epilogue.jpg",
            "afterword.jpg",
        ]));
        let mut pages: Vec<(String, f32)> = parsed
            .iter()
            .map(|p| (p.source.relative_path.clone(), p.page_number))
            .collect();
        pages.sort_by(|a, b| a.1.total_cmp(&b.1));
        // natord-sorted: afterword, epilogue, intro, prologue
        let names: Vec<&str> = pages.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["afterword.jpg", "epilogue.jpg", "intro.jpg", "prologue.jpg"]);
        // Page numbers are 1..4 in natord order.
        assert_eq!(pages.iter().map(|(_, p)| *p).collect::<Vec<_>>(), vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn mixed_parseable_and_unparseable_uses_natord_for_all() {
        // Mix: cover (unparseable for page), then numbered pages, then bonus
        // (unparseable). Because at least one filename is unparseable, ALL
        // files in this dir get natord positions.
        let parsed =
            resolver().resolve_batch(imgs(&["001.jpg", "002.jpg", "cover.jpg", "bonus.jpg"]));
        let parsed = ordered(parsed);
        // cover should be first (is_cover pins page to 0.0).
        assert!(parsed[0].is_cover);
    }

    #[test]
    fn trailing_number_in_filename() {
        // "story_007.jpg" — TRAILING_NUM_RE fallback extracts 7.
        let p = resolver().resolve(img("story_007.jpg")).unwrap();
        assert_eq!(p.page_number, 7.0);
    }

    // ───── series title nested with explicit labels ──────────────────────────

    #[test]
    fn series_title_nested_with_labels() {
        let parsed = resolver().resolve_batch(imgs(&[
            "Witch Hunter Robin/Vol 1/Ch 01/001.jpg",
            "Witch Hunter Robin/Vol 1/Ch 01/002.jpg",
            "Witch Hunter Robin/Vol 2/Ch 04/001.jpg",
        ]));
        let parsed = ordered(parsed);
        assert_eq!(parsed[0].identifier.volume, Some(1));
        assert_eq!(parsed[0].identifier.chapter, Some(1.0));
        assert_eq!(parsed[2].identifier.volume, Some(2));
        assert_eq!(parsed[2].identifier.chapter, Some(4.0));
    }

    // ───── filenames that look like chapter ranges ───────────────────────────

    #[test]
    fn filename_range_is_not_hakuneko() {
        // `001-002.jpg` as a FILENAME shouldn't fire HakunekoFolder (directory-only).
        // SPREAD_RE matches `[_-]` so "001-002" → Spread(1.0), "003-004" → Spread(3.0).
        let parsed = resolver().resolve_batch(imgs(&["001-002.jpg", "003-004.jpg"]));
        let pages: Vec<f32> = ordered(parsed).into_iter().map(|p| p.page_number).collect();
        assert_eq!(pages, vec![1.0, 3.0]);
    }
}
