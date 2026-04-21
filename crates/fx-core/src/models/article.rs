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
    /// Chapter/lecture scope. Used by notes (category='note') and questions
    /// (kind='question'). Reviews are always about the whole book/course and
    /// leave these NULL.
    #[sqlx(default)]
    pub book_chapter_id: Option<String>,
    #[sqlx(default)]
    pub course_session_id: Option<String>,
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
    /// Tag ids for concepts the article touches but does not teach.
    /// Mirrors `teaches` but carries no skill-mastery weight.
    #[serde(default)]
    pub related: Vec<String>,
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
    /// Chapter scope for questions or general articles that reference a book
    /// chapter. Notes carry this inside `CategoryMetadata::Note`; reviews
    /// leave it NULL (reviews are always whole-book).
    #[serde(default)]
    pub book_chapter_id: Option<String>,
    /// Lecture scope for questions or general articles that reference a
    /// course session. See `book_chapter_id` for the analogous note/review
    /// policy.
    #[serde(default)]
    pub course_session_id: Option<String>,
}

impl CreateArticle {
    /// `book_id` covers both Review (whole-book review) and Note (a note
    /// about that book). Either metadata variant carries it.
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
    pub fn target_course_id(&self) -> Option<&str> {
        match &self.metadata {
            Some(CategoryMetadata::Review { course_id, .. }) => course_id.as_deref(),
            Some(CategoryMetadata::Note   { course_id, .. }) => course_id.as_deref(),
            _ => None,
        }
    }
    /// Chapter scope. Only notes or top-level (questions/general articles)
    /// may set this — reviews are forced to None here, regardless of input.
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
    /// Lecture scope. Same review-forced-None policy as chapter scope.
    pub fn target_course_session_id(&self) -> Option<&str> {
        if self.category.as_deref() == Some("review") {
            return None;
        }
        if let Some(CategoryMetadata::Note { course_session_id, .. }) = &self.metadata {
            if course_session_id.is_some() {
                return course_session_id.as_deref();
            }
        }
        self.course_session_id.as_deref()
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
    #[serde(rename = "note")]
    Note {
        book_id: Option<String>,
        edition_id: Option<String>,
        course_id: Option<String>,
        /// Chapter-specific note. Only meaningful with `book_id` set.
        book_chapter_id: Option<String>,
        /// Lecture-specific note. Only meaningful with `course_id` set.
        course_session_id: Option<String>,
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
