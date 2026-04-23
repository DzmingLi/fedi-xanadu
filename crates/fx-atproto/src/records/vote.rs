//! Generated from lexicons/at.nightbo.vote.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! An upvote or downvote on an article, series chapter, or comment. `subject` is the voted-on at-uri (article or series); for a series chapter, `sectionRef` carries the chapter's TID.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.vote";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// When subject is a series, the chapter TID that was voted on.
    #[serde(rename = "sectionRef", default, skip_serializing_if = "Option::is_none")]
    pub section_ref: Option<String>,
    pub subject: String,
    pub value: i64,
}

