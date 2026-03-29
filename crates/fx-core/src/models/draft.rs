use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::tag::ArticlePrereq;
use crate::content::ContentFormat;

// ---- DB row ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Draft {
    pub id: String,
    pub did: String,
    pub title: String,
    pub description: String,
    pub content: String,
    pub content_format: ContentFormat,
    pub lang: String,
    pub license: String,
    pub tags: String,
    pub prereqs: String,
    pub at_uri: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- Request ----

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SaveDraft {
    pub title: String,
    pub description: Option<String>,
    pub content: String,
    pub content_format: ContentFormat,
    pub lang: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub prereqs: Vec<ArticlePrereq>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct UpdateDraft {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub content_format: Option<ContentFormat>,
    pub lang: Option<String>,
    pub license: Option<String>,
    pub tags: Option<Vec<String>>,
    pub prereqs: Option<Vec<ArticlePrereq>>,
}
