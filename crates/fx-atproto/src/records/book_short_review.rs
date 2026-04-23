//! Generated from lexicons/at.nightbo.book.shortReview.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A short review (brief text) of a book. Ratings live independently in at.nightbo.book.rating; a short review is just the text. rkey is the book_id so each user has at most one short review per book.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.book.shortReview";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub body: String,
    #[serde(rename = "bookId")]
    pub book_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Which edition the reviewer read (optional).
    #[serde(rename = "editionId", default, skip_serializing_if = "Option::is_none")]
    pub edition_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(rename = "updatedAt", default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
}

