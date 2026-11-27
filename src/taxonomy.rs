use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};

use crate::config::Paths;

const DEFAULT_TAXONOMY: &str = include_str!("../assets/taxonomy.default.toml");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagInfo {
    pub group: String,
    pub label_en: String,
    pub label_vi: String,
    pub desc_en: String,
    pub desc_vi: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Taxonomy {
    pub tags: BTreeMap<String, TagInfo>,
}

impl Taxonomy {
    /// Load `taxonomy.toml`; if absent, write the embedded default first.
    /// An unreadable or invalid file falls back to the embedded default
    /// (the file on disk is left untouched).
    pub fn load(paths: &Paths) -> Taxonomy {
        if !paths.taxonomy.exists() {
            let _ = fs::create_dir_all(&paths.home);
            let _ = fs::write(&paths.taxonomy, DEFAULT_TAXONOMY);
        }
        fs::read_to_string(&paths.taxonomy)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_else(Taxonomy::embedded_default)
    }

    pub fn contains(&self, tag: &str) -> bool {
        self.tags.contains_key(tag)
    }

    fn embedded_default() -> Taxonomy {
        toml::from_str(DEFAULT_TAXONOMY).expect("embedded default taxonomy is valid")
    }
}
