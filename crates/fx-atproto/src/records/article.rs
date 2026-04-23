//! Generated from lexicons/at.nightbo.article.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A standalone knowledge article (or question/answer/thought). Content is either backed by a pijul repository (for collaborative open-licensed works, addressed via `pijulRepoUrl`) or stored as blobs on the author's PDS (for All-Rights-Reserved and other non-forkable works, addressed via the `content` union). Chapters of a series are NOT published as at.nightbo.article records — the series record describes the whole work and the repo's chapter files are the authoritative structure.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.article";

/// Article content stored as one or more blobs on the author's PDS. Source files only — rendered artifacts (e.g. _rendered/*.svg, content.html) are server-derived and not included here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobContent {
    /// Relative path of the root source file within the file set, e.g. `content.typ` or `content.md`.
    pub entry: String,
    pub files: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobFile {
    pub blob: serde_json::Value,
    /// Relative path from the content root, e.g. `Figure/compare.pdf` or `main.bib`.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// Non-pijul content storage. When present, this is authoritative and `pijulRepoUrl` is legacy/historical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(rename = "contentFormat")]
    pub content_format: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Short summary / lede.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// SPDX-style license identifier. `All-Rights-Reserved` implies the article's content must not be stored in a forkable pijul repo — use `content` union with `#blobContent` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Public knot URL for the article's pijul repo. Required for historical continuity; for blob-backed content, may point to the original (now obsolete) pijul location or to an empty placeholder repo.
    #[serde(rename = "pijulRepoUrl")]
    pub pijul_repo_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub title: String,
}

