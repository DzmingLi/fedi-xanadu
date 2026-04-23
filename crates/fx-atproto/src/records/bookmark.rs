//! Generated from lexicons/at.nightbo.bookmark.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A reader's saved-for-later mark. `subject` is a standalone article's at-uri OR a series at-uri; for a series chapter, the extra `sectionRef` carries the chapter's TID. rkey is the chapter/article TID so add/remove/move are O(1).

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.bookmark";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// User-defined folder, e.g. '/' or '/to-read'.
    #[serde(rename = "folderPath")]
    pub folder_path: String,
    /// When subject is a series, the chapter TID identifying which chapter was bookmarked.
    #[serde(rename = "sectionRef", default, skip_serializing_if = "Option::is_none")]
    pub section_ref: Option<String>,
    /// Article or series being bookmarked.
    pub subject: String,
}

