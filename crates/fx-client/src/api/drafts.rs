//! Drafts API: list, save, update, delete, publish.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ClientResult, FxClient};
use crate::api::articles::Article;

// ---- Client-side response types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    pub id: String,
    pub did: String,
    pub title: String,
    pub summary: String,
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

// ---- Request types ----

#[derive(Debug, Clone, Serialize)]
pub struct SaveDraftInput {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub content: String,
    pub content_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub prereqs: Vec<crate::api::articles::ArticlePrereqInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateDraftInput {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prereqs: Option<Vec<crate::api::articles::ArticlePrereqInput>>,
}

impl FxClient {
    /// List all drafts for the current user.
    pub async fn list_drafts(&self) -> ClientResult<Vec<Draft>> {
        self.get("/drafts").await
    }

    /// Save a new draft. Requires auth.
    pub async fn save_draft(&self, input: &SaveDraftInput) -> ClientResult<Draft> {
        self.post("/drafts", input).await
    }

    /// Update an existing draft. Requires auth + ownership.
    pub async fn update_draft(&self, input: &UpdateDraftInput) -> ClientResult<Draft> {
        self.put(&format!("/drafts/{}", input.id), input).await
    }

    /// Delete a draft. Requires auth + ownership.
    pub async fn delete_draft(&self, id: &str) -> ClientResult<()> {
        self.delete_with_body(
            &format!("/drafts/{id}"),
            &serde_json::json!({ "id": id }),
        )
        .await
    }

    /// Publish a draft (convert to article). Requires auth + ownership.
    pub async fn publish_draft(&self, id: &str) -> ClientResult<Article> {
        self.post(
            &format!("/drafts/{id}/publish"),
            &serde_json::json!({ "id": id }),
        )
        .await
    }
}
