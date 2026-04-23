//! Generated from lexicons/at.nightbo.skilltree.json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.
//!
//! A curated DAG of tags that expresses a learner's (or author's) roadmap through a domain. Edges are parent→child relationships; prereqs are hard/soft dependencies on top of the hierarchy. Any structural edit re-puts the whole record so external AppViews always see a consistent snapshot. rkey is the tree's TID.

use serde::{Deserialize, Serialize};

/// NSID for this record type.
pub const NSID: &str = "at.nightbo.skilltree";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Parent→child tag edges forming the tree hierarchy.
    pub edges: Vec<serde_json::Value>,
    /// If this tree is a fork, the original tree's at-uri.
    #[serde(rename = "forkedFrom", default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,
    /// Cross-edge prerequisites that don't follow the parent/child hierarchy.
    pub prereqs: Vec<serde_json::Value>,
    /// Root tag of this tree, if any.
    #[serde(rename = "tagId", default, skip_serializing_if = "Option::is_none")]
    pub tag_id: Option<String>,
    pub title: String,
}

