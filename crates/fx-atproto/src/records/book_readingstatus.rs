//! Generated from lexicons/at.nightbo.book.readingstatus.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A reader's shelf status for a book (want_to_read / reading / finished / dropped). One record per (reader, book); rkey is the book_id.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.book.readingstatus";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "bookId")]
    pub book_id: String,
    #[serde(rename = "preferredEditionId", default, skip_serializing_if = "Option::is_none")]
    pub preferred_edition_id: Option<String>,
    /// Percentage read, 0-100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<i64>,
    pub status: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

