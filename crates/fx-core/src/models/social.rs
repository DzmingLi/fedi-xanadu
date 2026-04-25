use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Vote {
    pub at_uri: String,
    pub target_uri: String,
    pub did: String,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct UserSkill {
    pub did: String,
    /// The tag (concept) the user has lit. Every language label that
    /// belongs to this tag counts as lit — lighting "Calculus" also
    /// marks "高等数学" as lit because they share a tag.
    pub tag_id: String,
    pub status: String,
    pub lit_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct UserBookmark {
    pub did: String,
    pub article_uri: String,
    pub folder_path: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Comment {
    pub id: String,
    pub content_uri: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub parent_id: Option<String>,
    pub title: Option<String>,
    pub body: String,
    pub quote_text: Option<String>,
    pub section_ref: Option<String>,
    pub vote_score: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
