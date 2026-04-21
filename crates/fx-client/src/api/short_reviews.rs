//! Short reviews API: book + series short reviews (豆瓣-style).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ClientResult, FxClient};

// ---- Response types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookShortReview {
    pub id: String,
    pub did: String,
    pub book_id: String,
    pub edition_id: Option<String>,
    pub body: String,
    pub lang: Option<String>,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesShortReview {
    pub id: String,
    pub did: String,
    pub series_id: String,
    pub body: String,
    pub lang: Option<String>,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- Request types ----

#[derive(Debug, Clone, Serialize)]
pub struct UpsertBookShortReviewInput {
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpsertSeriesShortReviewInput {
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
}

// ---- FxClient methods ----

impl FxClient {
    pub async fn upsert_book_short_review(
        &self,
        book_id: &str,
        input: &UpsertBookShortReviewInput,
    ) -> ClientResult<BookShortReview> {
        self.post(&format!("/books/{book_id}/short-reviews"), input).await
    }

    pub async fn get_my_book_short_review(&self, book_id: &str) -> ClientResult<Option<BookShortReview>> {
        self.get(&format!("/books/{book_id}/short-reviews/my")).await
    }

    pub async fn list_book_short_reviews(&self, book_id: &str) -> ClientResult<Vec<BookShortReview>> {
        self.get(&format!("/books/{book_id}/short-reviews")).await
    }

    pub async fn delete_book_short_review(&self, book_id: &str) -> ClientResult<()> {
        self.delete_with_body(&format!("/books/{book_id}/short-reviews/my"), &serde_json::json!({})).await
    }

    pub async fn upsert_series_short_review(
        &self,
        series_id: &str,
        input: &UpsertSeriesShortReviewInput,
    ) -> ClientResult<SeriesShortReview> {
        self.post(&format!("/book-series/{series_id}/short-reviews"), input).await
    }

    pub async fn get_my_series_short_review(&self, series_id: &str) -> ClientResult<Option<SeriesShortReview>> {
        self.get(&format!("/book-series/{series_id}/short-reviews/my")).await
    }

    pub async fn list_series_short_reviews(&self, series_id: &str) -> ClientResult<Vec<SeriesShortReview>> {
        self.get(&format!("/book-series/{series_id}/short-reviews")).await
    }

    pub async fn delete_series_short_review(&self, series_id: &str) -> ClientResult<()> {
        self.delete_with_body(&format!("/book-series/{series_id}/short-reviews/my"), &serde_json::json!({})).await
    }
}

