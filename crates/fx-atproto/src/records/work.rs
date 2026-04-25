//! Generated from lexicons/at.nightbo.work.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A NightBoat knowledge work — article, book, lecture notes, tutorial — as a blob bundle. This record carries ONLY the bundle (every source file + asset as a blob on the author's PDS) and `createdAt`. All other metadata (title, description, tags, license, content format, entry file, whether it's a multi-chapter series, chapter ordering and anchors) lives inside the bundle in format-native slots: typst `#metadata(...) <nbt-article>` / `<nbt-series>` directives, markdown YAML frontmatter, HTML `<meta>` tags, or a `meta.yml` at bundle root. The server extracts these on ingestion.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.work";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub blob: serde_json::Value,
    /// Content type hint; authoritative copy is in the blob itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    /// Relative path from the bundle root, e.g. 'content.md', 'main.typ', 'chapters/01-intro.typ', 'common/main.bib', 'Figure/cover.pdf'.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Every source and asset file the work contains. For single-article works this is typically one entry file plus figures/bib. For series works, all chapter sources + shared files. The server identifies the entry, format, and structure from metadata inside the files.
    pub files: Vec<serde_json::Value>,
}

