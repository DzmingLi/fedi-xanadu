//! Articles API: list, get, create, update, delete, fork, content.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ClientResult, FxClient};

// ---- Client-side response types ----
// These mirror the server types but without sqlx dependencies.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub at_uri: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub content_hash: Option<String>,
    pub content_format: String,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleContent {
    pub source: String,
    pub html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticlePrereqRow {
    pub tag_id: String,
    pub prereq_type: String,
    pub tag_name: String,
    pub tag_names: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkWithTitle {
    pub fork_uri: String,
    pub forked_uri: String,
    pub vote_score: i32,
    pub title: String,
    pub did: String,
    pub author_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleFullResponse {
    pub article: Article,
    pub content: ArticleContent,
    pub prereqs: Vec<ArticlePrereqRow>,
    pub forks: Vec<ForkWithTitle>,
    pub votes: ArticleVoteSummary,
    pub series_context: Vec<serde_json::Value>,
    pub translations: Vec<Article>,
    pub my_vote: i32,
    pub is_bookmarked: bool,
    pub learned: bool,
    pub access_denied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleVoteSummary {
    pub score: i64,
    pub upvotes: i64,
    pub downvotes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTeachRow {
    pub content_uri: String,
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPrereqBulkRow {
    pub content_uri: String,
    pub tag_id: String,
    pub prereq_type: String,
}

// ---- Request types ----

#[derive(Debug, Clone, Serialize)]
pub struct CreateArticleInput {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub content: String,
    pub content_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restricted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition_id: Option<String>,
    pub tags: Vec<String>,
    pub prereqs: Vec<ArticlePrereqInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArticlePrereqInput {
    pub tag_id: String,
    pub prereq_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateArticleInput {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForkArticleInput {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_format: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConvertInput {
    pub content: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConvertOutput {
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageUploadResponse {
    pub filename: String,
}

// ---- Query helpers ----

#[derive(Serialize)]
struct UriQuery<'a> {
    uri: &'a str,
}

#[derive(Serialize)]
struct ListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<i64>,
}

#[derive(Serialize)]
struct TagArticlesQuery<'a> {
    tag_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
}

#[derive(Serialize)]
struct DidArticlesQuery<'a> {
    did: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
}

#[derive(Serialize)]
struct SearchQuery<'a> {
    q: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
}

#[derive(Serialize)]
struct BulkLimitQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
}

impl FxClient {
    // ---- Articles ----

    /// List articles with optional pagination.
    pub async fn list_articles(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> ClientResult<Vec<Article>> {
        self.get_with_query("/articles", &ListQuery { limit, offset })
            .await
    }

    /// Get a single article by AT URI.
    pub async fn get_article(&self, uri: &str) -> ClientResult<Article> {
        self.get_with_query("/articles/by-uri", &UriQuery { uri })
            .await
    }

    /// Get article content (source + rendered HTML).
    pub async fn get_article_content(&self, uri: &str) -> ClientResult<ArticleContent> {
        self.get_with_query("/articles/by-uri/content", &UriQuery { uri })
            .await
    }

    /// Get article prereqs.
    pub async fn get_article_prereqs(&self, uri: &str) -> ClientResult<Vec<ArticlePrereqRow>> {
        self.get_with_query("/articles/by-uri/prereqs", &UriQuery { uri })
            .await
    }

    /// Get article forks.
    pub async fn get_article_forks(&self, uri: &str) -> ClientResult<Vec<ForkWithTitle>> {
        self.get_with_query("/articles/by-uri/forks", &UriQuery { uri })
            .await
    }

    /// Get full article page data in a single request.
    pub async fn get_article_full(&self, uri: &str) -> ClientResult<ArticleFullResponse> {
        self.get_with_query("/articles/full", &UriQuery { uri })
            .await
    }

    /// Create a new article. Requires auth.
    pub async fn create_article(&self, input: &CreateArticleInput) -> ClientResult<Article> {
        self.post("/articles", input).await
    }

    /// Update an existing article. Requires auth + ownership.
    pub async fn update_article(&self, input: &UpdateArticleInput) -> ClientResult<Article> {
        self.put("/articles/update", input).await
    }

    /// Delete an article. Requires auth + ownership.
    pub async fn delete_article(&self, uri: &str) -> ClientResult<()> {
        self.delete_with_body("/articles/delete", &serde_json::json!({ "uri": uri }))
            .await
    }

    /// Fork an article, optionally converting format.
    pub async fn fork_article(&self, input: &ForkArticleInput) -> ClientResult<Article> {
        self.post("/articles/fork", input).await
    }

    /// Convert content between formats (no auth required).
    pub async fn convert_content(&self, input: &ConvertInput) -> ClientResult<ConvertOutput> {
        self.post("/articles/convert", input).await
    }

    /// Get articles by tag.
    pub async fn get_articles_by_tag(
        &self,
        tag_id: &str,
        limit: Option<i64>,
    ) -> ClientResult<Vec<Article>> {
        self.get_with_query("/articles/by-tag", &TagArticlesQuery { tag_id, limit })
            .await
    }

    /// Get articles by DID (author).
    pub async fn get_articles_by_did(
        &self,
        did: &str,
        limit: Option<i64>,
    ) -> ClientResult<Vec<Article>> {
        self.get_with_query("/articles/by-did", &DidArticlesQuery { did, limit })
            .await
    }

    /// Get translations of an article.
    pub async fn get_translations(&self, uri: &str) -> ClientResult<Vec<Article>> {
        self.get_with_query("/articles/translations", &UriQuery { uri })
            .await
    }

    /// Get all article teaches (bulk).
    pub async fn get_all_article_teaches(
        &self,
        limit: Option<i64>,
    ) -> ClientResult<Vec<ContentTeachRow>> {
        self.get_with_query("/articles/all-teaches", &BulkLimitQuery { limit })
            .await
    }

    /// Get all article prereqs (bulk).
    pub async fn get_all_article_prereqs(
        &self,
        limit: Option<i64>,
    ) -> ClientResult<Vec<ContentPrereqBulkRow>> {
        self.get_with_query("/articles/all-prereqs", &BulkLimitQuery { limit })
            .await
    }

    /// Search articles.
    pub async fn search_articles(
        &self,
        q: &str,
        limit: Option<i64>,
    ) -> ClientResult<Vec<Article>> {
        self.get_with_query("/search", &SearchQuery { q, limit })
            .await
    }

    /// Set restricted flag on an article.
    pub async fn set_restricted(&self, uri: &str, restricted: bool) -> ClientResult<()> {
        self.put_empty(
            "/articles/restricted",
            &serde_json::json!({ "uri": uri, "restricted": restricted }),
        )
        .await
    }

    /// Grant content access to a user.
    pub async fn grant_access(&self, uri: &str, grantee_did: &str) -> ClientResult<()> {
        self.post_empty_with_body(
            "/articles/access/grant",
            &serde_json::json!({ "uri": uri, "grantee_did": grantee_did }),
        )
        .await
    }

    /// Revoke content access from a user.
    pub async fn revoke_access(&self, uri: &str, grantee_did: &str) -> ClientResult<()> {
        self.delete_with_body(
            "/articles/access/revoke",
            &serde_json::json!({ "uri": uri, "grantee_did": grantee_did }),
        )
        .await
    }

    /// List access grants for an article.
    pub async fn list_access_grants(&self, uri: &str) -> ClientResult<Vec<serde_json::Value>> {
        self.get_with_query("/articles/access/list", &UriQuery { uri })
            .await
    }

}
