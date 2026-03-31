//! Series API: list, get, create, add/remove articles, prereqs, reorder.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ClientResult, FxClient};

// ---- Client-side response types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub long_description: Option<String>,
    pub parent_id: Option<String>,
    pub order_index: i32,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub lang: String,
    pub translation_group: Option<String>,
    pub category: String,
    #[serde(default = "default_split_level")]
    pub split_level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesListRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub long_description: Option<String>,
    pub parent_id: Option<String>,
    pub order_index: i32,
    pub created_by: String,
    pub author_handle: Option<String>,
    pub created_at: DateTime<Utc>,
    pub lang: String,
    pub translation_group: Option<String>,
    pub category: String,
    pub article_count: i64,
    pub child_count: i64,
    #[serde(default = "default_split_level")]
    pub split_level: i32,
}

fn default_split_level() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesDetailResponse {
    pub series: SeriesRow,
    pub articles: Vec<serde_json::Value>,
    pub prereqs: Vec<serde_json::Value>,
    pub children: Vec<SeriesRow>,
    pub translations: Vec<SeriesRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesTreeNode {
    pub series: SeriesRow,
    pub articles: Vec<serde_json::Value>,
    pub children: Vec<SeriesTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesArticleMemberRow {
    pub series_id: String,
    pub article_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesContextItem {
    pub series_id: String,
    pub series_title: String,
    pub total: i64,
    pub prev: Vec<serde_json::Value>,
    pub next: Vec<serde_json::Value>,
}

// ---- Request types ----

#[derive(Debug, Clone, Serialize)]
pub struct CreateSeriesInput {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

// ---- Query helpers ----

#[derive(Serialize)]
struct ListSeriesQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
}

#[derive(Serialize)]
struct UriQuery<'a> {
    uri: &'a str,
}

#[derive(Serialize)]
struct BulkLimitQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
}

impl FxClient {
    /// List all series.
    pub async fn list_series(&self, limit: Option<i64>) -> ClientResult<Vec<SeriesListRow>> {
        self.get_with_query("/series", &ListSeriesQuery { limit })
            .await
    }

    /// Get series detail (with articles, prereqs, children).
    pub async fn get_series_detail(&self, id: &str) -> ClientResult<SeriesDetailResponse> {
        self.get_with_query(&format!("/series/{id}"), &()).await
    }

    /// Get series tree (full hierarchy).
    pub async fn get_series_tree(&self, id: &str) -> ClientResult<SeriesTreeNode> {
        self.get_with_query(&format!("/series/{id}/tree"), &())
            .await
    }

    /// Create a new series. Requires auth.
    pub async fn create_series(&self, input: &CreateSeriesInput) -> ClientResult<SeriesRow> {
        self.post("/series", input).await
    }

    /// Add an article to a series. Requires auth + ownership.
    pub async fn add_series_article(
        &self,
        series_id: &str,
        article_uri: &str,
    ) -> ClientResult<()> {
        self.post_empty_with_body(
            &format!("/series/{series_id}/articles"),
            &serde_json::json!({
                "series_id": series_id,
                "article_uri": article_uri,
            }),
        )
        .await
    }

    /// Remove an article from a series. Requires auth + ownership.
    pub async fn remove_series_article(
        &self,
        series_id: &str,
        article_uri: &str,
    ) -> ClientResult<()> {
        self.delete_with_body(
            &format!("/series/{series_id}/articles/remove"),
            &serde_json::json!({
                "series_id": series_id,
                "article_uri": article_uri,
            }),
        )
        .await
    }

    /// Reorder articles within a series.
    pub async fn reorder_series_articles(
        &self,
        series_id: &str,
        article_uris: &[String],
    ) -> ClientResult<()> {
        self.put_empty(
            &format!("/series/{series_id}/articles/reorder"),
            &serde_json::json!({
                "series_id": series_id,
                "article_uris": article_uris,
            }),
        )
        .await
    }

    /// Reorder child series.
    pub async fn reorder_series_children(
        &self,
        parent_id: &str,
        child_ids: &[String],
    ) -> ClientResult<()> {
        self.put_empty(
            &format!("/series/{parent_id}/children/reorder"),
            &serde_json::json!({
                "parent_id": parent_id,
                "child_ids": child_ids,
            }),
        )
        .await
    }

    /// Add a prereq edge between articles in a series.
    pub async fn add_series_prereq(
        &self,
        series_id: &str,
        article_uri: &str,
        prereq_article_uri: &str,
    ) -> ClientResult<()> {
        self.post_empty_with_body(
            &format!("/series/{series_id}/prereqs"),
            &serde_json::json!({
                "series_id": series_id,
                "article_uri": article_uri,
                "prereq_article_uri": prereq_article_uri,
            }),
        )
        .await
    }

    /// Remove a prereq edge between articles in a series.
    pub async fn remove_series_prereq(
        &self,
        series_id: &str,
        article_uri: &str,
        prereq_article_uri: &str,
    ) -> ClientResult<()> {
        self.delete_with_body(
            &format!("/series/{series_id}/prereqs/remove"),
            &serde_json::json!({
                "series_id": series_id,
                "article_uri": article_uri,
                "prereq_article_uri": prereq_article_uri,
            }),
        )
        .await
    }

    /// Get series context for an article (navigation: prev/next in each series).
    pub async fn get_series_context(
        &self,
        uri: &str,
    ) -> ClientResult<Vec<SeriesContextItem>> {
        self.get_with_query("/series/context", &UriQuery { uri })
            .await
    }

    /// Get all series-article memberships (bulk, for dedup).
    pub async fn all_series_articles(
        &self,
        limit: Option<i64>,
    ) -> ClientResult<Vec<SeriesArticleMemberRow>> {
        self.get_with_query("/series/all-articles", &BulkLimitQuery { limit })
            .await
    }
}
