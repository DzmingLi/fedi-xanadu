//! Generated from lexicons/at.nightbo.book.rating.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A reader's numeric rating for a book (1–10 half-stars: 1=½, 10=5 stars). rkey is the book_id so a user has at most one rating per book.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.book.rating";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "bookId")]
    pub book_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub rating: i64,
    #[serde(rename = "updatedAt", default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

