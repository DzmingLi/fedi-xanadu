use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::tag::ArticlePrereq;
use crate::content::{ContentFormat, ContentKind};

// ---- DB row ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Article {
    pub at_uri: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub author_reputation: i32,
    pub kind: ContentKind,
    pub title: String,
    pub description: String,
    pub content_hash: Option<String>,
    pub content_format: ContentFormat,
    pub lang: String,
    pub translation_group: Option<String>,
    pub license: String,
    pub prereq_threshold: f64,
    pub category: String,
    pub question_uri: Option<String>,
    pub book_id: Option<String>,
    pub edition_id: Option<String>,
    pub answer_count: i32,
    pub restricted: bool,
    pub vote_score: i64,
    pub bookmark_count: i64,
    pub comment_count: i64,
    pub fork_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An author entry for an article, with verification status.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ArticleAuthor {
    pub author_did: String,
    pub author_handle: Option<String>,
    pub author_reputation: i32,
    pub position: Option<i16>,
    pub status: String,
    pub authorship_uri: Option<String>,
}

/// Paper-specific metadata (venue, DOI, arXiv, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PaperMetadata {
    pub article_uri: String,
    pub venue: Option<String>,
    pub venue_type: Option<String>,
    pub year: Option<i16>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub accepted: bool,
}

// ---- Request ----

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateArticle {
    pub title: String,
    pub description: Option<String>,
    pub content: String,
    pub content_format: ContentFormat,
    pub lang: Option<String>,
    pub license: Option<String>,
    pub translation_of: Option<String>,
    pub restricted: Option<bool>,
    pub category: Option<String>,
    pub book_id: Option<String>,
    pub edition_id: Option<String>,
    pub tags: Vec<String>,
    pub prereqs: Vec<ArticlePrereq>,
    /// If set, the article belongs to this series and its source is stored in the series repo.
    pub series_id: Option<String>,
    /// Paper metadata (venue, DOI, arXiv, etc.) — only for category=paper.
    #[serde(default)]
    pub paper: Option<CreatePaperMetadata>,
    /// Experience metadata (target, result, etc.) — only for category=experience.
    #[serde(default)]
    pub experience: Option<CreateExperienceMetadata>,
    /// Co-author DIDs (the creator is always included automatically).
    #[serde(default)]
    pub authors: Vec<String>,
    /// Handles to invite to answer this question (only used when kind=Question).
    #[serde(default)]
    pub invites: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreatePaperMetadata {
    pub venue: Option<String>,
    pub venue_type: Option<String>,
    pub year: Option<i16>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    #[serde(default)]
    pub accepted: bool,
}

/// Experience post metadata (postgrad, interview, competition, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ExperienceMetadata {
    pub article_uri: String,
    pub kind: Option<String>,
    pub target: Option<String>,
    pub year: Option<i16>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateExperienceMetadata {
    pub kind: Option<String>,
    pub target: Option<String>,
    pub year: Option<i16>,
    pub result: Option<String>,
}

// ---- Response ----

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ArticleContent {
    pub source: String,
    pub html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ArticlePrereqRow {
    pub tag_id: String,
    pub prereq_type: String,
    pub tag_name: String,
    #[ts(type = "Record<string, string>")]

    pub tag_names: sqlx::types::Json<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ForkWithTitle {
    pub fork_uri: String,
    pub forked_uri: String,
    pub vote_score: i32,
    pub title: String,
    pub did: String,
    pub author_handle: Option<String>,
}
