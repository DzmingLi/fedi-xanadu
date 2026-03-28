//! Tags API: list, get, search, create.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ClientResult, FxClient};

// ---- Client-side response types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub names: HashMap<String, String>,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

// ---- Request types ----

#[derive(Debug, Clone, Serialize)]
pub struct CreateTagInput {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub names: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateTagNamesInput {
    pub id: String,
    pub names: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetTeachInput {
    pub content_uri: String,
    pub tag_id: String,
}

// ---- Query helpers ----

#[derive(Serialize)]
struct ListTagsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
}

#[derive(Serialize)]
struct SearchTagsQuery<'a> {
    q: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
}

impl FxClient {
    /// List all tags.
    pub async fn list_tags(&self, limit: Option<i64>) -> ClientResult<Vec<Tag>> {
        self.get_with_query("/tags", &ListTagsQuery { limit }).await
    }

    /// Get a tag by ID.
    pub async fn get_tag(&self, id: &str) -> ClientResult<Tag> {
        self.get_with_query(&format!("/tags/{id}"), &()) .await
    }

    /// Search tags by query string.
    pub async fn search_tags(&self, q: &str, limit: Option<i64>) -> ClientResult<Vec<Tag>> {
        self.get_with_query("/tags/search", &SearchTagsQuery { q, limit })
            .await
    }

    /// Create a new tag. Requires auth.
    pub async fn create_tag(&self, input: &CreateTagInput) -> ClientResult<Tag> {
        self.post("/tags", input).await
    }

    /// Update tag names (admin-only, requires admin secret).
    pub async fn update_tag_names(&self, input: &UpdateTagNamesInput) -> ClientResult<Tag> {
        self.put(&format!("/tags/{}/names", input.id), input).await
    }

    /// Set content teaches relationship.
    pub async fn set_teach(&self, content_uri: &str, tag_id: &str) -> ClientResult<()> {
        self.post_empty_with_body(
            "/tags/teach",
            &SetTeachInput {
                content_uri: content_uri.to_string(),
                tag_id: tag_id.to_string(),
            },
        )
        .await
    }
}
