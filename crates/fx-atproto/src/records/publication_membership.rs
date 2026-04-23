//! Generated from lexicons/at.nightbo.publication.membership.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! Member's half of the bilateral handshake — written under the MEMBER's DID to confirm they accept the invitation listed in the publication record.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.publication.membership";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "acceptedAt")]
    pub accepted_at: String,
    /// Publication slug, duplicated here for server-side lookup convenience.
    #[serde(rename = "publicationSlug")]
    pub publication_slug: String,
    /// AT URI of the publication record (owner's DID). Optional — some publications may not yet have a persisted PDS record at time of acceptance.
    #[serde(rename = "publicationUri", default, skip_serializing_if = "Option::is_none")]
    pub publication_uri: Option<String>,
    pub role: String,
}

