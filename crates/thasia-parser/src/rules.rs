use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    /// If true, the engine falls back to mapping directory depths to identifiers
    /// when explicit labels (Vol/Ch) are absent.
    pub enable_depth_fallback: bool,

    /// Maps directory depth to an identifier.
    /// Example: ["volume", "chapter", "page"] means depth 0 = volume, depth 1 = chapter.
    pub depth_mapping: Vec<String>,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            enable_depth_fallback: true,
            depth_mapping: vec!["volume".into(), "chapter".into(), "page".into()],
        }
    }
}
