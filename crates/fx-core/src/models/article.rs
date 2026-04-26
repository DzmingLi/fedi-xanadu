use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::tag::ArticlePrereq;
use crate::content::{ContentFormat, ContentKind};

// ---- Identity ----
//
// After the translation rewrite, every article is keyed by
// `(repo_uri, source_path)`. The old `at_uri` single-column identity is now
// per-language (one `at_uri` per `article_localizations` row) and may be
// NULL (series chapters publish no per-chapter record; Q&A is server-only).
// Services use `ArticleKey` as the stable, language-independent handle.

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ArticleKey {
    pub repo_uri: String,
    pub source_path: String,
}

impl ArticleKey {
    pub fn new(repo_uri: impl Into<String>, source_path: impl Into<String>) -> Self {
        Self {
            repo_uri: repo_uri.into(),
            source_path: source_path.into(),
        }
    }

    /// Matches the `article_uri(p_repo_uri, p_source_path)` SQL function:
    /// used as a single-string key in generic URI-keyed tables (comments,
    /// votes, bookmarks, content registry).
    pub fn synthetic_uri(&self) -> String {
        format!("nightboat://article/{}/{}", self.repo_uri, self.source_path)
    }
}

// ---- Display struct: article + currently-selected localization + joins ----

/// Aggregated view used by most API responses: language-independent article
/// fields merged with one specific localization (source-language by default)
/// plus joined author info and aggregate counts.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Article {
    // Identity
    pub repo_uri: String,
    pub source_path: String,

    // Author (joined)
    pub author_did: String,
    pub author_handle: Option<String>,
    pub author_display_name: Option<String>,
    pub author_avatar: Option<String>,
    pub author_reputation: i32,

    // Per-article (stable across languages)
    pub kind: ContentKind,
    pub category: String,
    pub license: String,
    pub prereq_threshold: f64,
    pub restricted: bool,
    pub answer_count: i32,
    #[sqlx(default)]
    pub cover_url: Option<String>,
    #[sqlx(default)]
    pub cover_file: Option<String>,
    pub question_repo_uri: Option<String>,
    pub question_source_path: Option<String>,
    pub book_id: Option<String>,
    pub edition_id: Option<String>,
    #[sqlx(default)]
    pub term_id: Option<String>,
    #[sqlx(default)]
    pub book_chapter_id: Option<String>,
    #[sqlx(default)]
    pub term_session_id: Option<String>,

    // Selected localization (source-language by default in the base query)
    pub lang: String,
    /// ATProto record URI for this specific localization. NULL for series
    /// chapters and for server-only Q&A.
    pub at_uri: Option<String>,
    pub file_path: String,
    pub title: String,
    pub summary: String,
    #[sqlx(default)]
    pub summary_html: String,
    pub content_hash: Option<String>,
    pub content_format: ContentFormat,
    #[sqlx(default)]
    pub translator_did: Option<String>,

    // Paper metadata (joined; None for non-papers)
    #[sqlx(default)]
    pub paper_venue: Option<String>,
    #[sqlx(default)]
    pub paper_year: Option<i16>,
    #[sqlx(default)]
    pub paper_accepted: Option<bool>,

    // Aggregates
    #[ts(type = "number")]
    pub vote_score: i64,
    #[ts(type = "number")]
    pub bookmark_count: i64,
    #[ts(type = "number")]
    pub comment_count: i64,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Article {
    pub fn key(&self) -> ArticleKey {
        ArticleKey::new(&self.repo_uri, &self.source_path)
    }

    /// Synthetic URI for generic URI-keyed lookups.
    pub fn synthetic_uri(&self) -> String {
        self.key().synthetic_uri()
    }
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

/// Paper-specific metadata. Keyed by composite article key now.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PaperMetadata {
    pub repo_uri: String,
    pub source_path: String,
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
    /// Repo-relative path of the source article this content translates.
    pub translation_of: Option<String>,
    pub restricted: Option<bool>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub prereqs: Vec<ArticlePrereq>,
    #[serde(default)]
    pub related: Vec<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    pub series_id: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub invites: Vec<String>,
    #[serde(default)]
    pub metadata: Option<CategoryMetadata>,
    #[serde(default)]
    pub book_chapter_id: Option<String>,
    #[serde(default)]
    pub term_session_id: Option<String>,
}

impl CreateArticle {
    pub fn target_book_id(&self) -> Option<&str> {
        match &self.metadata {
            Some(CategoryMetadata::Review { book_id, .. }) => book_id.as_deref(),
            Some(CategoryMetadata::Note   { book_id, .. }) => book_id.as_deref(),
            _ => None,
        }
    }
    pub fn target_edition_id(&self) -> Option<&str> {
        match &self.metadata {
            Some(CategoryMetadata::Review { edition_id, .. }) => edition_id.as_deref(),
            Some(CategoryMetadata::Note   { edition_id, .. }) => edition_id.as_deref(),
            _ => None,
        }
    }
    pub fn target_term_id(&self) -> Option<&str> {
        match &self.metadata {
            Some(CategoryMetadata::Review { term_id, .. }) => term_id.as_deref(),
            Some(CategoryMetadata::Note   { term_id, .. }) => term_id.as_deref(),
            _ => None,
        }
    }
    pub fn target_book_chapter_id(&self) -> Option<&str> {
        if self.category.as_deref() == Some("review") {
            return None;
        }
        if let Some(CategoryMetadata::Note { book_chapter_id, .. }) = &self.metadata {
            if book_chapter_id.is_some() {
                return book_chapter_id.as_deref();
            }
        }
        self.book_chapter_id.as_deref()
    }
    pub fn target_term_session_id(&self) -> Option<&str> {
        if self.category.as_deref() == Some("review") {
            return None;
        }
        if let Some(CategoryMetadata::Note { term_session_id, .. }) = &self.metadata {
            if term_session_id.is_some() {
                return term_session_id.as_deref();
            }
        }
        self.term_session_id.as_deref()
    }
}

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
        term_id: Option<String>,
    },
    #[serde(rename = "note")]
    Note {
        book_id: Option<String>,
        edition_id: Option<String>,
        term_id: Option<String>,
        book_chapter_id: Option<String>,
        term_session_id: Option<String>,
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

/// Experience post metadata. Composite key.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ExperienceMetadata {
    pub repo_uri: String,
    pub source_path: String,
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

