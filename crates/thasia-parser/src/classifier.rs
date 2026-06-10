//! Per-component pattern classifier. Every path component (directory or
//! filename stem) is classified into exactly one of these categories using
//! anchored regexes — no global token-stream matching, so series titles like
//! "Spider-Verse" can't leak Volume/Page tokens.

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq)]
pub enum Component {
    /// `Vol 3`, `volume_3`, `V3`, `V-3` etc.
    VolumeMarker(u32),
    /// `Ch 12`, `chapter-12.5`, `C12`, `c.12` etc.
    ChapterMarker(f32),
    /// Filename `p3`, `page_3`, `p-3.5` etc.
    PageMarker(f32),
    /// Directory `001-002` — Hakuneko-style "(volume)-(chapter)".
    HakunekoFolder(u32, f32),
    /// Filename `001_002` — double-page spread; primary = first number.
    Spread(f32),
    /// Pure-numeric component `001`, `12.5` — meaning is positional
    /// (volume / chapter / page depending on depth).
    PureNumber(f32),
    /// Component contains "cover" (case-insensitive). Filename-only.
    Cover,
    /// Anything else — series titles, scanlator names, "raws", "extras"…
    /// Contributes nothing to the resolved fields.
    Noise,
}

/// Position of the component in the path — used to gate filename-only vs
/// directory-only patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    Directory,
    Filename, // already stripped of extension
}

// Anchored, case-insensitive. Each pattern is intentionally narrow — false
// positives are far more harmful than false negatives because the assembly
// phase has fallbacks (depth_mapping, natord-position).
static VOLUME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?i)v(?:ol(?:ume)?)?[ ._-]*(\d+)$").expect("valid volume regex")
});
static CHAPTER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?i)c(?:h(?:apter)?)?[ ._-]*(\d+(?:\.\d+)?)$").expect("valid chapter regex")
});
static PAGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?i)p(?:age)?[ ._-]*(\d+(?:\.\d+)?)$").expect("valid page regex")
});
static HAKUNEKO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+)-(\d+(?:\.\d+)?)$").expect("valid hakuneko regex"));
static SPREAD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+)[_-](\d+)$").expect("valid spread regex"));
static PURE_NUM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+(?:\.\d+)?)$").expect("valid pure number regex"));
/// Permissive fallback for filenames whose stem ends with a number: `bonus_002`,
/// `chapter-end-007`, `intro1`. Avoids forcing every weirdly-named file into
/// natord-fallback when there's an obvious trailing page number.
static TRAILING_NUM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+(?:\.\d+)?)$").expect("valid trailing number regex"));

/// Classify one path component. Patterns are checked in priority order, most
/// specific first. The first match wins; further patterns are not tried.
pub fn classify(component: &str, kind: ComponentKind) -> Component {
    let s = component.trim();
    if s.is_empty() {
        return Component::Noise;
    }

    // Cover is only meaningful on a filename; in a directory it'd just be the
    // "cover" sub-folder name which is rare and confusing if treated specially.
    if kind == ComponentKind::Filename && s.to_lowercase().contains("cover") {
        return Component::Cover;
    }

    if let Some(c) = VOLUME_RE.captures(s)
        && let Ok(n) = c[1].parse::<u32>()
    {
        return Component::VolumeMarker(n);
    }
    if let Some(c) = CHAPTER_RE.captures(s)
        && let Ok(n) = c[1].parse::<f32>()
    {
        return Component::ChapterMarker(n);
    }
    if let Some(c) = PAGE_RE.captures(s)
        && let Ok(n) = c[1].parse::<f32>()
    {
        return Component::PageMarker(n);
    }

    // HakunekoFolder only fires for directories — for filenames the same shape
    // `001-002` is ambiguous (could be a chapter range) so we treat it as Noise.
    if kind == ComponentKind::Directory
        && let Some(c) = HAKUNEKO_RE.captures(s)
    {
        let v = c[1].parse::<u32>().ok();
        let ch = c[2].parse::<f32>().ok();
        if let (Some(v), Some(ch)) = (v, ch) {
            return Component::HakunekoFolder(v, ch);
        }
    }

    // Spread only fires for filenames — `001_002` as a directory name is too
    // ambiguous to classify.
    if kind == ComponentKind::Filename
        && let Some(c) = SPREAD_RE.captures(s)
        && let Ok(n) = c[1].parse::<f32>()
    {
        return Component::Spread(n);
    }

    if let Some(c) = PURE_NUM_RE.captures(s)
        && let Ok(n) = c[1].parse::<f32>()
    {
        return Component::PureNumber(n);
    }

    // Trailing-number fallback: only for filenames, only as a last resort. This
    // catches things like `bonus_007` or `chapter-end-12` without polluting
    // directory classification.
    if kind == ComponentKind::Filename
        && let Some(c) = TRAILING_NUM_RE.captures(s)
        && let Ok(n) = c[1].parse::<f32>()
    {
        return Component::PageMarker(n);
    }

    Component::Noise
}

#[cfg(test)]
mod tests {
    use super::*;
    use ComponentKind::*;

    #[test]
    fn vol_label_variants() {
        assert_eq!(classify("Vol 3", Directory), Component::VolumeMarker(3));
        assert_eq!(classify("volume_3", Directory), Component::VolumeMarker(3));
        assert_eq!(classify("V3", Directory), Component::VolumeMarker(3));
        assert_eq!(classify("v-3", Directory), Component::VolumeMarker(3));
        assert_eq!(classify("V.3", Directory), Component::VolumeMarker(3));
    }

    #[test]
    fn chapter_label_variants() {
        assert_eq!(classify("Ch 12", Directory), Component::ChapterMarker(12.0));
        assert_eq!(
            classify("chapter_10.5", Directory),
            Component::ChapterMarker(10.5)
        );
        assert_eq!(classify("C7", Directory), Component::ChapterMarker(7.0));
    }

    #[test]
    fn page_label_variants() {
        assert_eq!(classify("p3", Filename), Component::PageMarker(3.0));
        assert_eq!(classify("page_007", Filename), Component::PageMarker(7.0));
        assert_eq!(classify("p35.5", Filename), Component::PageMarker(35.5));
        assert_eq!(classify("p35.6", Filename), Component::PageMarker(35.6));
    }

    #[test]
    fn hakuneko_folder() {
        assert_eq!(
            classify("001-002", Directory),
            Component::HakunekoFolder(1, 2.0)
        );
        // Decimal chapter number (e.g. chapter 10.5 in volume 1)
        assert_eq!(
            classify("001-10.5", Directory),
            Component::HakunekoFolder(1, 10.5)
        );
        // Same shape as filename is ambiguous → Noise.
        assert_ne!(
            classify("001-002", Filename),
            Component::HakunekoFolder(1, 2.0)
        );
    }

    #[test]
    fn spread_filename() {
        assert_eq!(classify("001_002", Filename), Component::Spread(1.0));
        assert_eq!(classify("003_004", Filename), Component::Spread(3.0));
    }

    #[test]
    fn pure_numeric_pages() {
        assert_eq!(classify("001", Filename), Component::PureNumber(1.0));
        assert_eq!(classify("12.5", Filename), Component::PureNumber(12.5));
    }

    #[test]
    fn cover_detection() {
        assert_eq!(classify("cover", Filename), Component::Cover);
        assert_eq!(classify("Cover", Filename), Component::Cover);
        assert_eq!(classify("front_cover", Filename), Component::Cover);
        // Directories with "cover" stay literal — too rare and confusing.
        assert_ne!(classify("cover", Directory), Component::Cover);
    }

    #[test]
    fn series_titles_are_noise() {
        assert_eq!(classify("Spider-Verse", Directory), Component::Noise);
        assert_eq!(classify("Witch Hunter Robin", Directory), Component::Noise);
        assert_eq!(classify("Vampire Knight", Directory), Component::Noise);
        // Title with "v" letter shouldn't fire VolumeMarker.
        assert_eq!(classify("Verse", Directory), Component::Noise);
    }

    #[test]
    fn ambiguous_title_with_numbers_stays_noise() {
        // "My Manga - 5 (oneshot)" — current parser misfires; new parser sees
        // this as Noise (doesn't anchor-match any pattern).
        assert_eq!(classify("My Manga - 5", Directory), Component::Noise);
        assert_eq!(
            classify("Series Volume 2 special", Directory),
            Component::Noise
        );
    }

    #[test]
    fn trailing_number_fallback_filename_only() {
        // Permissive fallback: `bonus_007.jpg` → PageMarker(7)
        assert_eq!(classify("bonus_007", Filename), Component::PageMarker(7.0));
        // But NOT for directories — they get classified as Noise.
        assert_eq!(classify("bonus_007", Directory), Component::Noise);
    }

    #[test]
    fn empty_and_whitespace() {
        assert_eq!(classify("", Filename), Component::Noise);
        assert_eq!(classify("   ", Filename), Component::Noise);
    }
}
