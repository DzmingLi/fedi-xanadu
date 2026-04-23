//! Generated from lexicons/at.nightbo.learned.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A reader's mark that they have completed / read something. `subject` is a standalone article's at-uri OR a series at-uri; for a series chapter, `sectionRef` carries the chapter's TID. rkey is the chapter/article TID.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.learned";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "learnedAt")]
    pub learned_at: String,
    /// When subject is a series, the chapter TID completed.
    #[serde(rename = "sectionRef", default, skip_serializing_if = "Option::is_none")]
    pub section_ref: Option<String>,
    /// Article or series the reader completed.
    pub subject: String,
}

