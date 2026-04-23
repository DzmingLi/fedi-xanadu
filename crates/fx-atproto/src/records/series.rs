//! Generated from lexicons/at.nightbo.series.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A multi-chapter work (book, lecture notes, tutorial) whose content lives in a single pijul repository. Chapter division and order come from the repo's layout (`chapters/*.{typ,md,...}`) — there are no per-chapter at-protocol records. External AppViews clone `pijulRepoUrl` to get the complete series plus version history. Interactions on individual chapters (vote, bookmark, comment) reference the series URI plus a `sectionRef` (chapter TID) to retain per-chapter granularity without fragmenting PDS writes.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.series";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(rename = "coverUrl", default, skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(rename = "longDescription", default, skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    /// Public knot URL hosting the series repo, e.g. https://knot.example.com/did:web:alice/series_<seriesId>
    #[serde(rename = "pijulRepoUrl")]
    pub pijul_repo_url: String,
    /// Stable series id, used as the rkey.
    #[serde(rename = "seriesId")]
    pub series_id: String,
    /// Short description / blurb.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<String>>,
}

