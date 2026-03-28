//! Books API: list, get, create, update, editions, chapters, ratings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ClientResult, FxClient};
use crate::api::articles::Article;

// ---- Client-side response types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub description: String,
    pub cover_url: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookEdition {
    pub id: String,
    pub book_id: String,
    pub title: String,
    pub lang: String,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookChapter {
    pub id: String,
    pub book_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub order_index: i32,
    pub article_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookRatingStats {
    pub avg_rating: f64,
    pub rating_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingStatus {
    pub book_id: String,
    pub user_did: String,
    pub status: String,
    pub progress: i16,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterProgress {
    pub book_id: String,
    pub chapter_id: String,
    pub user_did: String,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDetail {
    pub book: Book,
    pub editions: Vec<BookEdition>,
    pub chapters: Vec<BookChapter>,
    pub reviews: Vec<Article>,
    pub review_count: usize,
    pub rating: BookRatingStats,
    pub my_rating: Option<i16>,
    pub my_reading_status: Option<ReadingStatus>,
    pub my_chapter_progress: Vec<ChapterProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookEditLog {
    pub id: String,
    pub book_id: String,
    pub editor_did: String,
    pub editor_handle: Option<String>,
    pub old_data: serde_json::Value,
    pub new_data: serde_json::Value,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

// ---- Request types ----

#[derive(Debug, Clone, Serialize)]
pub struct CreateBookInput {
    pub title: String,
    pub authors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateBookInput {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddEditionInput {
    pub book_id: String,
    pub title: String,
    pub lang: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateChapterInput {
    pub book_id: String,
    pub chapter: CreateChapterData,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateChapterData {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub order_index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub article_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateBookInput {
    pub book_id: String,
    pub rating: i16,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetReadingStatusInput {
    pub book_id: String,
    pub status: String,
    #[serde(default)]
    pub progress: i16,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChapterProgressInput {
    pub book_id: String,
    pub chapter_id: String,
    pub completed: bool,
}

// ---- Query helpers ----

#[derive(Serialize)]
struct ListBooksQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<i64>,
}

impl FxClient {
    /// List books with optional pagination.
    pub async fn list_books(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> ClientResult<Vec<Book>> {
        self.get_with_query("/books", &ListBooksQuery { limit, offset })
            .await
    }

    /// Get full book detail (editions, chapters, reviews, ratings, user state).
    pub async fn get_book(&self, id: &str) -> ClientResult<BookDetail> {
        self.get_with_query(&format!("/books/{id}"), &()).await
    }

    /// Create a new book. Requires auth.
    pub async fn create_book(&self, input: &CreateBookInput) -> ClientResult<Book> {
        self.post("/books", input).await
    }

    /// Update a book. Requires auth.
    pub async fn update_book(&self, input: &UpdateBookInput) -> ClientResult<Book> {
        self.put(&format!("/books/{}", input.id), input).await
    }

    /// Add an edition to a book. Requires auth.
    pub async fn add_edition(&self, input: &AddEditionInput) -> ClientResult<BookEdition> {
        self.post(&format!("/books/{}/editions", input.book_id), input)
            .await
    }

    /// List chapters for a book.
    pub async fn list_chapters(&self, book_id: &str) -> ClientResult<Vec<BookChapter>> {
        self.get_with_query(
            &format!("/books/{book_id}/chapters"),
            &serde_json::json!({ "book_id": book_id }),
        )
        .await
    }

    /// Create a chapter. Requires auth.
    pub async fn create_chapter(&self, input: &CreateChapterInput) -> ClientResult<BookChapter> {
        self.post(&format!("/books/{}/chapters", input.book_id), input)
            .await
    }

    /// Delete a chapter. Requires auth.
    pub async fn delete_chapter(&self, book_id: &str, chapter_id: &str) -> ClientResult<()> {
        self.delete_with_body(
            &format!("/books/{book_id}/chapters/delete"),
            &serde_json::json!({ "id": chapter_id }),
        )
        .await
    }

    /// Set chapter progress (completed/not). Requires auth.
    pub async fn set_chapter_progress(
        &self,
        input: &ChapterProgressInput,
    ) -> ClientResult<()> {
        self.post_empty_with_body(
            &format!("/books/{}/chapters/progress", input.book_id),
            input,
        )
        .await
    }

    /// Rate a book (1-10). Requires auth.
    pub async fn rate_book(
        &self,
        book_id: &str,
        rating: i16,
    ) -> ClientResult<BookRatingStats> {
        self.post(
            &format!("/books/{book_id}/rate"),
            &RateBookInput {
                book_id: book_id.to_string(),
                rating,
            },
        )
        .await
    }

    /// Set reading status for a book. Requires auth.
    pub async fn set_reading_status(
        &self,
        book_id: &str,
        status: &str,
        progress: i16,
    ) -> ClientResult<()> {
        self.post_empty_with_body(
            &format!("/books/{book_id}/reading-status"),
            &SetReadingStatusInput {
                book_id: book_id.to_string(),
                status: status.to_string(),
                progress,
            },
        )
        .await
    }

    /// Remove reading status for a book. Requires auth.
    pub async fn remove_reading_status(&self, book_id: &str) -> ClientResult<()> {
        self.delete_with_body(
            &format!("/books/{book_id}/reading-status"),
            &serde_json::json!({ "book_id": book_id }),
        )
        .await
    }

    /// Get edit history for a book.
    pub async fn get_book_edit_history(&self, book_id: &str) -> ClientResult<Vec<BookEditLog>> {
        self.get_with_query(
            &format!("/books/{book_id}/history"),
            &serde_json::json!({ "id": book_id }),
        )
        .await
    }
}
