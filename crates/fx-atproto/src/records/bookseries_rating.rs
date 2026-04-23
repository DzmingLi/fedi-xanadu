//! Generated from lexicons/at.nightbo.bookseries.rating.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A reader's numeric rating for a book series (1–10 half-stars: 1=½, 10=5 stars). rkey is the series_id so a user has at most one rating per series.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.bookseries.rating";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub rating: i64,
    #[serde(rename = "seriesId")]
    pub series_id: String,
    #[serde(rename = "updatedAt", default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

