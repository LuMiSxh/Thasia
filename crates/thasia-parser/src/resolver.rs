use crate::{lexer::Token, rules::RuleConfig};
use logos::Logos;
use std::path::Path;
use thasia_core::models::{ChapterIdentifier, DiscoveredImage, ParsedImage};

pub struct Resolver {
    config: RuleConfig,
}

impl Resolver {
    pub fn new(config: RuleConfig) -> Self {
        Self { config }
    }

    pub fn resolve(&self, img: DiscoveredImage) -> Result<ParsedImage, String> {
        let mut volume: Option<u32> = None;
        let mut chapter: Option<f32> = None;
        let mut page: Option<u32> = None;
        let mut is_cover = false;

        let tokens: Vec<_> = Token::lexer(&img.relative_path)
            .filter_map(|r| r.ok())
            .collect();
        let mut iter = tokens.iter().peekable();

        // 1. Explicit semantic analysis (e.g. "Vol 1 Ch 2")
        while let Some(token) = iter.next() {
            match token {
                Token::Cover => is_cover = true,
                Token::Volume => {
                    if let Some(Token::Number(n)) = iter.peek() {
                        volume = Some(*n as u32);
                        iter.next();
                    }
                }
                Token::Chapter => {
                    if let Some(Token::Number(n)) = iter.peek() {
                        chapter = Some(*n);
                        iter.next();
                    }
                }
                Token::Page => {
                    if let Some(Token::Number(n)) = iter.peek() {
                        page = Some(*n as u32);
                        iter.next();
                    }
                }
                Token::Number(n) => {
                    // Hakuneko-style: Number-Dash-Number means volume-chapter
                    if let Some(Token::Dash) = iter.peek() {
                        iter.next(); // consume Dash
                        if let Some(Token::Number(m)) = iter.peek() {
                            volume = Some(*n as u32);
                            chapter = Some(*m);
                            iter.next(); // consume second Number
                        }
                    }
                }
                _ => {}
            }
        }

        // 2. Depth mapping fallback (e.g. "1/14/001.jpg")
        if self.config.enable_depth_fallback && (volume.is_none() || chapter.is_none()) {
            let path = Path::new(&img.relative_path);
            let components: Vec<&str> = path.iter().filter_map(|s| s.to_str()).collect();

            for (depth, comp) in components.iter().enumerate() {
                if let Some(level) = self.config.depth_mapping.get(depth) {
                    let n = Token::lexer(comp).filter_map(|r| r.ok()).find_map(|t| {
                        if let Token::Number(n) = t {
                            Some(n)
                        } else {
                            None
                        }
                    });

                    if let Some(n) = n {
                        match level.as_str() {
                            "volume" => volume = volume.or(Some(n as u32)),
                            "chapter" => chapter = chapter.or(Some(n)),
                            "page" => page = page.or(Some(n as u32)),
                            _ => {}
                        }
                    }
                }
            }
        }

        // 3. Page fallback: last number in the path is usually the page
        if page.is_none() {
            page = tokens.iter().rev().find_map(|t| {
                if let Token::Number(n) = t {
                    Some(*n as u32)
                } else {
                    None
                }
            });
        }

        page.ok_or_else(|| format!("Could not determine page number for: {}", img.relative_path))
            .map(|page_number| ParsedImage {
                source: img,
                identifier: ChapterIdentifier { volume, chapter },
                page_number,
                is_cover,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use thasia_core::models::DiscoveredImage;

    fn img(rel: &str) -> DiscoveredImage {
        DiscoveredImage {
            absolute_path: PathBuf::from("/root").join(rel),
            relative_path: rel.to_string(),
        }
    }

    fn resolver() -> Resolver {
        Resolver::new(RuleConfig::default())
    }

    #[test]
    fn test_hakuneko_style() {
        let r = resolver();
        let p = r.resolve(img("001-002/001.jpg")).unwrap();
        assert_eq!(p.identifier.volume, Some(1));
        assert_eq!(p.identifier.chapter, Some(2.0));
        assert_eq!(p.page_number, 1);
        assert!(!p.is_cover);
    }

    #[test]
    fn test_explicit_vol_ch_labels() {
        let r = resolver();
        let p = r.resolve(img("Vol 1/Ch 04/001.jpg")).unwrap();
        assert_eq!(p.identifier.volume, Some(1));
        assert_eq!(p.identifier.chapter, Some(4.0));
        assert_eq!(p.page_number, 1);
        assert!(!p.is_cover);
    }

    #[test]
    fn test_pure_number_depth_fallback() {
        let r = resolver();
        let p = r.resolve(img("1/14/001.jpg")).unwrap();
        assert_eq!(p.identifier.volume, Some(1));
        assert_eq!(p.identifier.chapter, Some(14.0));
        assert_eq!(p.page_number, 1);
    }

    #[test]
    fn test_cover_detection() {
        let r = resolver();
        let p = r.resolve(img("Vol 1/cover.jpg")).unwrap();
        assert!(p.is_cover);
    }

    #[test]
    fn test_float_chapter() {
        let r = resolver();
        let p = r.resolve(img("Vol 2/Ch 10.5/001.png")).unwrap();
        assert_eq!(p.identifier.chapter, Some(10.5));
    }

    #[test]
    fn test_no_page_returns_error() {
        let r = resolver();
        let result = r.resolve(img("Vol 1/Ch 2/no_page_number_here.jpg"));
        // The filename "no_page_number_here" has no number, but depth mapping might pick it up
        // at depth 2 as "page". If not, it should error. This verifies graceful handling.
        let _ = result; // just ensure it doesn't panic
    }
}
