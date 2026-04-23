//! Generated from lexicons/at.nightbo.publication.entry.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! Cross-post record: the content author declares that their article or series is being published to the named publication. Written under the CONTENT AUTHOR's DID so that authors retain agency over where their content appears.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.publication.entry";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "contentKind")]
    pub content_kind: String,
    /// For articles: their at-uri. For series: the series id (series don't have at-uris today).
    #[serde(rename = "contentUri")]
    pub content_uri: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "publicationSlug")]
    pub publication_slug: String,
    #[serde(rename = "publicationUri", default, skip_serializing_if = "Option::is_none")]
    pub publication_uri: Option<String>,
}

