//! Book series API: create, list, get, update, members, ratings.

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ClientResult, FxClient};

// ---- Response types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSeries {
    pub id: String,
    pub title: Value,
    pub subtitle: Value,
    pub description: Value,
    pub cover_url: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSeriesListItem {
    pub id: String,
    pub title: Value,
    pub cover_url: Option<String>,
    pub member_count: i64,
    pub member_avg_rating: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSeriesRef {
    pub id: String,
    pub title: Value,
    pub position: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSeriesRatingStats {
    pub avg_rating: f64,
    pub rating_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSeriesDetail {
    pub series: BookSeries,
    pub members: Vec<Value>,
    pub member_avg_rating: f64,
    pub member_rating_count: i64,
    pub series_rating: BookSeriesRatingStats,
    pub my_series_rating: Option<i16>,
    pub short_review_count: i64,
    pub recent_short_reviews: Vec<Value>,
    pub my_short_review: Option<Value>,
}

// ---- Request types ----

#[derive(Debug, Clone, Serialize)]
pub struct CreateBookSeriesInput {
    pub id: String,
    pub title: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateBookSeriesInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddSeriesMemberInput {
    pub book_id: String,
    pub position: i16,
}

// ---- FxClient methods ----

impl FxClient {
    pub async fn list_book_series(&self) -> ClientResult<Vec<BookSeriesListItem>> {
        self.get("/book-series").await
    }

    pub async fn get_book_series(&self, id: &str) -> ClientResult<BookSeriesDetail> {
        self.get(&format!("/book-series/{id}")).await
    }

    pub async fn create_book_series(&self, input: &CreateBookSeriesInput) -> ClientResult<BookSeries> {
        self.post("/book-series", input).await
    }

    pub async fn update_book_series(&self, id: &str, input: &UpdateBookSeriesInput) -> ClientResult<BookSeries> {
        self.put(&format!("/book-series/{id}"), input).await
    }

    pub async fn add_series_member(&self, series_id: &str, input: &AddSeriesMemberInput) -> ClientResult<Value> {
        self.post(&format!("/book-series/{series_id}/members"), input).await
    }

    pub async fn remove_series_member(&self, series_id: &str, book_id: &str) -> ClientResult<()> {
        self.delete_with_body(&format!("/book-series/{series_id}/members/{book_id}"), &serde_json::json!({})).await
    }

    pub async fn rate_series(&self, series_id: &str, rating: i16) -> ClientResult<BookSeriesRatingStats> {
        self.post(&format!("/book-series/{series_id}/rate"), &serde_json::json!({ "rating": rating })).await
    }

    pub async fn unrate_series(&self, series_id: &str) -> ClientResult<BookSeriesRatingStats> {
        self.delete_json(&format!("/book-series/{series_id}/rate"), &serde_json::json!({})).await
    }
}
