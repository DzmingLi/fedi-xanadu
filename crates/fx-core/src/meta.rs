//! Article and series metadata stored as `meta.json` in pijul repos.
//!
//! This makes metadata (tags, prereqs, title, description, license, etc.)
//! version-controlled alongside content — forkable, diffable, cherry-pickable.
//! The database remains an indexed cache of this data.

use serde::{Deserialize, Serialize};

/// Article metadata stored in `meta.json` at the pijul repo root.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArticleMeta {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub prereqs: Vec<PrereqEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrereqEntry {
    pub tag_id: String,
    pub prereq_type: String,
}

/// Series metadata stored in `meta.json` at the series repo root.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SeriesMeta {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
}

pub const META_FILENAME: &str = "meta.json";

/// Write meta.json to a directory (repo root).
pub fn write_meta_file(dir: &std::path::Path, meta: &ArticleMeta) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(meta).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(dir.join(META_FILENAME), json)
}

/// Read meta.json from a directory.
pub fn read_meta_file(dir: &std::path::Path) -> Option<ArticleMeta> {
    let path = dir.join(META_FILENAME);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Write series meta.json.
pub fn write_series_meta_file(dir: &std::path::Path, meta: &SeriesMeta) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(meta).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(dir.join(META_FILENAME), json)
}

/// Read series meta.json.
pub fn read_series_meta_file(dir: &std::path::Path) -> Option<SeriesMeta> {
    let path = dir.join(META_FILENAME);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Build ArticleMeta from create inputs.
pub fn article_meta_from_create(
    title: &str,
    description: Option<&str>,
    tags: &[String],
    prereqs: &[crate::models::ArticlePrereq],
    license: Option<&str>,
    lang: Option<&str>,
    category: Option<&str>,
    content_format: &str,
) -> ArticleMeta {
    ArticleMeta {
        title: title.to_string(),
        description: description.map(|s| s.to_string()),
        tags: tags.to_vec(),
        prereqs: prereqs.iter().map(|p| PrereqEntry {
            tag_id: p.tag_id.clone(),
            prereq_type: p.prereq_type.as_str().to_string(),
        }).collect(),
        license: license.map(|s| s.to_string()),
        lang: lang.map(|s| s.to_string()),
        category: category.map(|s| s.to_string()),
        content_format: Some(content_format.to_string()),
    }
}
