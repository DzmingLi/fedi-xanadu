//! Generated from lexicons/at.nightbo.bookseries.shortReview.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A short review (brief text) of a book series. Same shape as at.nightbo.book.shortReview, scoped to a series. Ratings are separate (at.nightbo.bookseries.rating). rkey is the series_id so each user has at most one short review per series.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.bookseries.shortReview";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(rename = "seriesId")]
    pub series_id: String,
    #[serde(rename = "updatedAt", default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
}

