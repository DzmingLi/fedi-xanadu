//! Generated from lexicons/at.nightbo.skill.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A user's self-declared mastery of a tag (topic). One record per (user, tag); rkey is the tag id. `status` distinguishes mastery levels. Deleting the record = the user no longer claims this skill.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.skill";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub status: String,
    #[serde(rename = "tagId")]
    pub tag_id: String,
    #[serde(rename = "updatedAt", default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

