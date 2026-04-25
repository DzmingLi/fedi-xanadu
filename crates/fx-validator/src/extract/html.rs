//! Read article-level metadata from a bundle's `meta.json` sibling.
//!
//! HTML articles don't store nightboat metadata inside the source — the
//! HTML stays free of `<meta name="nightboat:..">` pollution and instead
//! ships a sibling `meta.json` next to `content.html`. This module parses
//! that JSON. Counterpart to [`fx_validator::inject`]'s HTML path which
//! lives in `fx-server` (it depends on `CreateArticle`, which would create
//! a circular dep here).

use crate::{FileMeta, Format, ValidationError};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
struct Raw {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    cover: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    related: Vec<String>,
}

/// Parse `meta.json` content into a [`FileMeta`]. `path` is the path of the
/// HTML article this metadata describes (typically `content.html`), not the
/// path of the meta.json file itself.
pub fn extract(path: &str, meta_json: &str) -> Result<FileMeta, ValidationError> {
    let raw: Raw = serde_json::from_str(meta_json).map_err(|e| ValidationError::MetadataParseError {
        path: path.to_string(),
        reason: format!("meta.json parse error: {e}"),
    })?;
    let lang = raw.lang.ok_or_else(|| ValidationError::MissingLang { path: path.to_string() })?;
    Ok(FileMeta {
        path: path.to_string(),
        format: Format::Md, // HTML isn't in the Format enum; treat as md for FileMeta-as-projection use
        lang,
        translation_of: None,
        title: raw.title,
        abstract_: raw.description,
        tags: raw.tags,
        translator: None,
        translation_notes: None,
        license: raw.license,
        category: raw.category,
        cover: raw.cover,
        related: raw.related,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_full_payload() {
        let json = r#"{
            "title": "Hi",
            "description": "An intro",
            "lang": "zh",
            "category": "lecture",
            "license": "CC-BY-4.0",
            "cover": "Figure/cover.png",
            "tags": ["calc", "analysis"],
            "related": ["history"]
        }"#;
        let m = extract("content.html", json).unwrap();
        assert_eq!(m.title.as_deref(), Some("Hi"));
        assert_eq!(m.abstract_.as_deref(), Some("An intro"));
        assert_eq!(m.lang, "zh");
        assert_eq!(m.category.as_deref(), Some("lecture"));
        assert_eq!(m.license.as_deref(), Some("CC-BY-4.0"));
        assert_eq!(m.cover.as_deref(), Some("Figure/cover.png"));
        assert_eq!(m.tags, vec!["calc".to_string(), "analysis".to_string()]);
        assert_eq!(m.related, vec!["history".to_string()]);
    }

    #[test]
    fn missing_lang_errors() {
        let json = r#"{ "title": "Hi" }"#;
        assert!(matches!(
            extract("content.html", json).unwrap_err(),
            ValidationError::MissingLang { .. }
        ));
    }

    #[test]
    fn malformed_errors() {
        let json = "{not json";
        assert!(matches!(
            extract("content.html", json).unwrap_err(),
            ValidationError::MetadataParseError { .. }
        ));
    }
}
