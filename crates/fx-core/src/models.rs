use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub names: sqlx::types::Json<HashMap<String, String>>,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTag {
    pub id: String,
    pub name: String,
    pub names: Option<HashMap<String, String>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum PrereqType {
    Required,
    Recommended,
    Suggested,
}

impl PrereqType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Recommended => "recommended",
            Self::Suggested => "suggested",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Article {
    pub at_uri: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub content_hash: Option<String>,
    pub content_format: String,
    pub lang: String,
    pub translation_group: Option<String>,
    pub license: String,
    pub prereq_threshold: f64,
    pub question_uri: Option<String>,
    pub answer_count: i32,
    pub vote_score: i64,
    pub bookmark_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArticle {
    pub title: String,
    pub description: Option<String>,
    pub content: String,
    pub content_format: String,
    pub lang: Option<String>,
    pub license: Option<String>,
    pub translation_of: Option<String>,
    pub tags: Vec<String>,
    pub prereqs: Vec<ArticlePrereq>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticlePrereq {
    pub tag_id: String,
    pub prereq_type: PrereqType,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserSkill {
    pub did: String,
    pub tag_id: String,
    pub status: String,
    pub lit_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Draft {
    pub id: String,
    pub did: String,
    pub title: String,
    pub description: String,
    pub content: String,
    pub content_format: String,
    pub lang: String,
    pub license: String,
    pub tags: String,
    pub prereqs: String,
    pub at_uri: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveDraft {
    pub title: String,
    pub description: Option<String>,
    pub content: String,
    pub content_format: String,
    pub lang: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub prereqs: Vec<ArticlePrereq>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDraft {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub content_format: Option<String>,
    pub lang: Option<String>,
    pub license: Option<String>,
    pub tags: Option<Vec<String>>,
    pub prereqs: Option<Vec<ArticlePrereq>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Vote {
    pub at_uri: String,
    pub target_uri: String,
    pub did: String,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Fork {
    pub fork_uri: String,
    pub source_uri: String,
    pub forked_uri: String,
    pub pijul_patch_hash: Option<String>,
    pub vote_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleContent {
    pub source: String,
    pub html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ArticlePrereqRow {
    pub tag_id: String,
    pub prereq_type: String,
    pub tag_name: String,
    pub tag_names: sqlx::types::Json<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ForkWithTitle {
    pub fork_uri: String,
    pub forked_uri: String,
    pub vote_score: i32,
    pub title: String,
    pub did: String,
    pub author_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserBookmark {
    pub did: String,
    pub article_uri: String,
    pub folder_path: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Comment {
    pub id: String,
    pub content_uri: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub parent_id: Option<String>,
    pub body: String,
    pub quote_text: Option<String>,
    pub vote_score: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prereq_type_as_str() {
        assert_eq!(PrereqType::Required.as_str(), "required");
        assert_eq!(PrereqType::Recommended.as_str(), "recommended");
        assert_eq!(PrereqType::Suggested.as_str(), "suggested");
    }

    #[test]
    fn prereq_type_serde_roundtrip() {
        let val = PrereqType::Required;
        let json = serde_json::to_string(&val).unwrap();
        assert_eq!(json, "\"required\"");
        let back: PrereqType = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "required");
    }

    #[test]
    fn create_article_serde() {
        let json = r#"{
            "title": "Test",
            "content": "body",
            "content_format": "typst",
            "tags": ["rust", "math"],
            "prereqs": [{"tag_id": "linear-algebra", "prereq_type": "required"}]
        }"#;
        let input: CreateArticle = serde_json::from_str(json).unwrap();
        assert_eq!(input.title, "Test");
        assert_eq!(input.tags.len(), 2);
        assert_eq!(input.prereqs[0].prereq_type.as_str(), "required");
    }

    #[test]
    fn save_draft_serde() {
        let json = r#"{
            "title": "Draft",
            "content": "wip",
            "content_format": "markdown",
            "tags": [],
            "prereqs": []
        }"#;
        let draft: SaveDraft = serde_json::from_str(json).unwrap();
        assert_eq!(draft.title, "Draft");
        assert!(draft.lang.is_none());
    }

    #[test]
    fn update_draft_all_optional() {
        let json = r#"{"id": "abc123"}"#;
        let update: UpdateDraft = serde_json::from_str(json).unwrap();
        assert_eq!(update.id, "abc123");
        assert!(update.title.is_none());
        assert!(update.content.is_none());
        assert!(update.tags.is_none());
    }
}
