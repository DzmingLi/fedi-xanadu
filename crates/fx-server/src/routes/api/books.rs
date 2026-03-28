use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::book_service;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use super::{Auth, WriteAuth, tid};

// --- List books ---

#[derive(serde::Deserialize)]
pub struct ListBooksQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_books(
    State(state): State<AppState>,
    Query(q): Query<ListBooksQuery>,
) -> ApiResult<Json<Vec<book_service::Book>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let books = book_service::list_books(&state.pool, limit, offset).await?;
    Ok(Json(books))
}

// --- Get book detail ---

#[derive(serde::Deserialize)]
pub struct BookIdQuery {
    pub id: String,
}

#[derive(serde::Serialize)]
pub struct BookDetail {
    pub book: book_service::Book,
    pub editions: Vec<book_service::BookEdition>,
    pub reviews: Vec<fx_core::models::Article>,
    pub review_count: usize,
}

pub async fn get_book(
    State(state): State<AppState>,
    Query(q): Query<BookIdQuery>,
) -> ApiResult<Json<BookDetail>> {
    let book = book_service::get_book(&state.pool, &q.id).await?;
    let editions = book_service::list_editions(&state.pool, &q.id).await?;
    let reviews = book_service::get_book_reviews(&state.pool, &q.id, 100, 0).await?;
    let review_count = reviews.len();
    Ok(Json(BookDetail { book, editions, reviews, review_count }))
}

// --- Create book ---

pub async fn create_book(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<book_service::CreateBook>,
) -> ApiResult<(StatusCode, Json<book_service::Book>)> {
    if input.title.trim().is_empty() {
        return Err(AppError(fx_core::Error::BadRequest("title required".into())));
    }
    let id = format!("bk-{}", tid());
    let book = book_service::create_book(&state.pool, &id, &input, &user.did).await?;
    Ok((StatusCode::CREATED, Json(book)))
}

// --- Update book ---

#[derive(serde::Deserialize)]
pub struct UpdateBookInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub edit_summary: Option<String>,
}

pub async fn update_book(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<UpdateBookInput>,
) -> ApiResult<Json<book_service::Book>> {
    // Record edit history before applying
    let old = book_service::get_book(&state.pool, &input.id).await?;
    let old_snapshot = serde_json::json!({
        "title": old.title,
        "description": old.description,
        "cover_url": old.cover_url,
    });

    book_service::update_book(
        &state.pool,
        &input.id,
        input.title.as_deref(),
        input.description.as_deref(),
        input.cover_url.as_deref(),
    ).await?;

    // Save edit log
    let edit_id = tid();
    sqlx::query(
        "INSERT INTO book_edit_log (id, book_id, editor_did, old_data, new_data, summary) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&edit_id)
    .bind(&input.id)
    .bind(&user.did)
    .bind(&old_snapshot)
    .bind(&serde_json::json!({
        "title": input.title,
        "description": input.description,
        "cover_url": input.cover_url,
    }))
    .bind(input.edit_summary.as_deref().unwrap_or(""))
    .execute(&state.pool)
    .await?;

    let book = book_service::get_book(&state.pool, &input.id).await?;
    Ok(Json(book))
}

// --- Add edition ---

#[derive(serde::Deserialize)]
pub struct AddEditionInput {
    pub book_id: String,
    #[serde(flatten)]
    pub edition: book_service::CreateEdition,
}

pub async fn add_edition(
    State(state): State<AppState>,
    Auth(_user): Auth,
    Json(input): Json<AddEditionInput>,
) -> ApiResult<(StatusCode, Json<book_service::BookEdition>)> {
    // Verify book exists
    let _ = book_service::get_book(&state.pool, &input.book_id).await?;
    let id = format!("ed-{}", tid());
    let edition = book_service::create_edition(&state.pool, &id, &input.book_id, &input.edition).await?;
    Ok((StatusCode::CREATED, Json(edition)))
}

// --- Book edit history ---

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct BookEditLog {
    pub id: String,
    pub book_id: String,
    pub editor_did: String,
    pub editor_handle: Option<String>,
    pub old_data: serde_json::Value,
    pub new_data: serde_json::Value,
    pub summary: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_edit_history(
    State(state): State<AppState>,
    Query(q): Query<BookIdQuery>,
) -> ApiResult<Json<Vec<BookEditLog>>> {
    let rows = sqlx::query_as::<_, BookEditLog>(
        "SELECT l.id, l.book_id, l.editor_did, p.handle AS editor_handle, \
                l.old_data, l.new_data, l.summary, l.created_at \
         FROM book_edit_log l \
         LEFT JOIN profiles p ON l.editor_did = p.did \
         WHERE l.book_id = $1 \
         ORDER BY l.created_at DESC \
         LIMIT 100",
    )
    .bind(&q.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}
