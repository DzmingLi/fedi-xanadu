use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::tag::ArticlePrereq;
use crate::content::{Category, ContentFormat, ContentKind};

// ---- DB row ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Article {
    pub at_uri: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub kind: ContentKind,
    pub title: String,
    pub description: String,
    pub content_hash: Option<String>,
    pub content_format: ContentFormat,
    pub lang: String,
    pub translation_group: Option<String>,
    pub license: String,
    pub prereq_threshold: f64,
    pub category: Category,
    pub question_uri: Option<String>,
    pub book_id: Option<String>,
    pub edition_id: Option<String>,
    pub answer_count: i32,
    pub restricted: bool,
    pub vote_score: i64,
    pub bookmark_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub category: Option<Category>,
    pub book_id: Option<String>,
    pub edition_id: Option<String>,
    pub tags: Vec<String>,
    pub prereqs: Vec<ArticlePrereq>,
    /// If set, the article belongs to this series and its source is stored in the series repo.
    pub series_id: Option<String>,
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
