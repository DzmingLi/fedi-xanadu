//! Generated from lexicons/at.nightbo.publication.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A Medium-style content channel (专栏). Owned by a single DID (the record author). Multi-editor support is achieved via bilateral handshake: the owner lists members here, and each member creates a at.nightbo.publication.membership record under their own DID to confirm.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.publication";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "coverUrl", default, skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Locale-keyed description map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<serde_json::Value>>,
    /// URL-safe identifier, [a-zA-Z0-9_-], used as the rkey.
    pub slug: String,
    /// Locale-keyed title map, e.g. {"en": "AI Weekly", "zh": "AI 周刊"}.
    pub title: serde_json::Value,
}

