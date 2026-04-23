//! Generated from lexicons/at.nightbo.publication.follow.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A follower's subscription record — one per (follower, publication) pair. Written under the follower's DID.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.publication.follow";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "publicationSlug")]
    pub publication_slug: String,
    #[serde(rename = "publicationUri", default, skip_serializing_if = "Option::is_none")]
    pub publication_uri: Option<String>,
}

