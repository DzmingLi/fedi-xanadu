//! Generated from lexicons/at.nightbo.comment.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A threaded discussion comment on an article (or another comment). Stored under the commenter's DID; `parent` is the replied-to comment URI when this is a reply.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.comment";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// When set, this comment is a reply to the referenced comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Optional quote from the subject being commented on.
    #[serde(rename = "quoteText", default, skip_serializing_if = "Option::is_none")]
    pub quote_text: Option<String>,
    /// Anchor / section identifier within the subject.
    #[serde(rename = "sectionRef", default, skip_serializing_if = "Option::is_none")]
    pub section_ref: Option<String>,
    /// The article (or content) this comment is attached to.
    pub subject: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "updatedAt", default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

