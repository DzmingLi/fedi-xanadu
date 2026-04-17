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
    pub author_display_name: Option<String>,
    pub author_avatar: Option<String>,
    pub author_reputation: i32,
    pub kind: ContentKind,
    pub title: String,
    pub summary: String,
    #[sqlx(default)]
    pub summary_html: String,
    #[sqlx(default)]
    pub cover_url: Option<String>,
    /// Paper metadata (joined). `paper_accepted` drives the "accepted venue"
    /// badge next to the title on cards. None for non-papers.
    #[sqlx(default)]
    pub paper_venue: Option<String>,
    #[sqlx(default)]
    pub paper_year: Option<i16>,
    #[sqlx(default)]
    pub paper_accepted: Option<bool>,
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
    pub author_did: Option<String>,
    pub author_name: Option<String>,
    pub author_handle: Option<String>,
    pub author_display_name: Option<String>,
    pub author_avatar: Option<String>,
    pub author_reputation: i32,
    pub position: Option<i16>,
    pub role: String,
    pub is_corresponding: bool,
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
    pub summary: Option<String>,
    pub content: String,
    pub content_format: ContentFormat,
    pub lang: Option<String>,
    pub license: Option<String>,
    pub translation_of: Option<String>,
    pub restricted: Option<bool>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub prereqs: Vec<ArticlePrereq>,
    /// If set, the article belongs to this series and its source is stored in the series repo.
    pub series_id: Option<String>,
    /// Co-author DIDs (the creator is always included automatically).
    #[serde(default)]
    pub authors: Vec<String>,
    /// Handles to invite to answer this question (only used when kind=Question).
    #[serde(default)]
    pub invites: Vec<String>,
    /// Category-specific metadata.
    #[serde(default)]
    pub metadata: Option<CategoryMetadata>,
}

impl CreateArticle {
    /// Extract book_id from Review metadata.
    pub fn review_book_id(&self) -> Option<&str> {
        match &self.metadata {
            Some(CategoryMetadata::Review { book_id, .. }) => book_id.as_deref(),
            _ => None,
        }
    }
    /// Extract edition_id from Review metadata.
    pub fn review_edition_id(&self) -> Option<&str> {
        match &self.metadata {
            Some(CategoryMetadata::Review { edition_id, .. }) => edition_id.as_deref(),
            _ => None,
        }
    }
}

/// Category-specific metadata — tagged union, only one variant per article.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
#[serde(tag = "type")]
pub enum CategoryMetadata {
    #[serde(rename = "paper")]
    Paper(CreatePaperMetadata),
    #[serde(rename = "review")]
    Review {
        book_id: Option<String>,
        edition_id: Option<String>,
        course_id: Option<String>,
    },
    #[serde(rename = "experience")]
    Experience(CreateExperienceMetadata),
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

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateExperienceMetadata {
    pub kind: Option<String>,
    pub target: Option<String>,
    pub year: Option<i16>,
    pub result: Option<String>,
}

/// Experience post metadata (DB row).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ExperienceMetadata {
    pub article_uri: String,
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
