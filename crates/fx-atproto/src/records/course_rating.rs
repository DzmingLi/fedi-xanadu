//! Generated from lexicons/at.nightbo.course.rating.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A learner's numeric rating for a course (1–10 half-stars). rkey is the course_id so one rating per (user, course).

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.course.rating";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "courseId")]
    pub course_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub rating: i64,
    #[serde(rename = "updatedAt", default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

