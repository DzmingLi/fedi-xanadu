use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTag {
    pub id: String,
    pub name: String,
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
    pub title: String,
    pub description: String,
    pub content_hash: Option<String>,
    pub content_format: String,
    pub lang: String,
    pub translation_group: Option<String>,
    pub license: String,
    pub prereq_threshold: f64,
    pub vote_score: i64,
    pub bookmark_count: i64,
    pub created_at: String,
    pub updated_at: String,
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
    pub lit_at: String,
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
    pub created_at: String,
    pub updated_at: String,
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
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Comment {
    pub id: String,
    pub article_uri: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub parent_id: Option<String>,
    pub body: String,
    pub vote_score: i64,
    pub created_at: String,
    pub updated_at: String,
}
