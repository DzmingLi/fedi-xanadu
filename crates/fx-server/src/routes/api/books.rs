use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, Response, header},
};
use fx_core::services::{book_service, skill_service};

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::{Auth, MaybeAuth, WriteAuth};
use fx_core::util::tid;

// --- List books ---

#[derive(serde::Deserialize)]
pub struct ListBooksQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_books(
    State(state): State<AppState>,
    Query(q): Query<ListBooksQuery>,
) -> ApiResult<Json<Vec<book_service::BookListItem>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let books = book_service::list_books_rich(&state.pool, limit, offset).await?;
    Ok(Json(books))
}

// --- Get book detail ---

#[derive(serde::Serialize)]
pub struct BookDetail {
    pub book: book_service::Book,
    pub editions: Vec<book_service::BookEdition>,
    pub chapters: Vec<book_service::BookChapterWithTags>,
    pub reviews: Vec<fx_core::models::Article>,
    pub review_count: usize,
    pub rating: book_service::BookRatingStats,
    pub my_rating: Option<i16>,
    pub my_reading_status: Option<book_service::ReadingStatus>,
    pub my_chapter_progress: Vec<book_service::ChapterProgress>,
    pub tags: Vec<String>,
    pub prereqs: Vec<String>,
}

pub async fn get_book(
    State(state): State<AppState>,
    Path(id): Path<String>,
    MaybeAuth(user): MaybeAuth,
) -> ApiResult<Json<BookDetail>> {
    let book = book_service::get_book(&state.pool, &id).await?;
    let editions = book_service::list_editions(&state.pool, &id).await?;
    let chapters = book_service::list_chapters_with_tags(&state.pool, &id).await?;
    let reviews = book_service::get_book_reviews(&state.pool, &id, 100, 0).await?;
    let review_count = reviews.len();
    let rating = book_service::get_rating_stats(&state.pool, &id).await?;
    let (my_rating, my_reading_status, my_chapter_progress) = if let Some(ref u) = user {
        let r = book_service::get_user_rating(&state.pool, &id, &u.did).await?;
        let s = book_service::get_reading_status(&state.pool, &id, &u.did).await?;
        let cp = book_service::list_chapter_progress(&state.pool, &id, &u.did).await?;
        (r, s, cp)
    } else {
        (None, None, vec![])
    };
    let content_uri = format!("book:{id}");
    let tags: Vec<String> = sqlx::query_scalar(
        "SELECT tag_id FROM content_teaches WHERE content_uri = $1 ORDER BY tag_id",
    )
    .bind(&content_uri)
    .fetch_all(&state.pool)
    .await?;
    let prereqs: Vec<String> = sqlx::query_scalar(
        "SELECT tag_id FROM content_prereqs WHERE content_uri = $1 ORDER BY tag_id",
    )
    .bind(&content_uri)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(BookDetail { book, editions, chapters, reviews, review_count, rating, my_rating, my_reading_status, my_chapter_progress, tags, prereqs }))
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
    Path(id): Path<String>,
    Auth(user): Auth,
    Json(mut input): Json<UpdateBookInput>,
) -> ApiResult<Json<book_service::Book>> {
    input.id = id;
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

pub async fn add_edition(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Auth(_user): Auth,
    Json(input): Json<book_service::CreateEdition>,
) -> ApiResult<(StatusCode, Json<book_service::BookEdition>)> {
    // Verify book exists
    let _ = book_service::get_book(&state.pool, &book_id).await?;
    let id = format!("ed-{}", tid());
    let edition = book_service::create_edition(&state.pool, &id, &book_id, &input).await?;
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
    Path(id): Path<String>,
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
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

// --- Rate book ---

#[derive(serde::Deserialize)]
pub struct RateBookInput {
    pub rating: i16,
}

pub async fn rate_book(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Auth(user): Auth,
    Json(input): Json<RateBookInput>,
) -> ApiResult<Json<book_service::BookRatingStats>> {
    if !(1..=10).contains(&input.rating) {
        return Err(AppError(fx_core::Error::BadRequest("rating must be 1-10 (half-stars)".into())));
    }
    let _ = book_service::get_book(&state.pool, &book_id).await?;
    book_service::rate_book(&state.pool, &book_id, &user.did, input.rating).await?;
    let stats = book_service::get_rating_stats(&state.pool, &book_id).await?;
    Ok(Json(stats))
}

// --- Set reading status ---

#[derive(serde::Deserialize)]
pub struct SetReadingStatusInput {
    pub status: String,
    #[serde(default)]
    pub progress: i16,
}

pub async fn set_reading_status(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Auth(user): Auth,
    Json(input): Json<SetReadingStatusInput>,
) -> ApiResult<StatusCode> {
    let valid = ["want_to_read", "reading", "finished", "dropped"];
    if !valid.contains(&input.status.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest("status must be want_to_read, reading, finished, or dropped".into())));
    }
    let progress = input.progress.clamp(0, 100);
    let _ = book_service::get_book(&state.pool, &book_id).await?;
    book_service::set_reading_status(&state.pool, &book_id, &user.did, &input.status, progress).await?;

    // Auto-learn: when finished, mark book's teaches tags as mastered
    if input.status == "finished" {
        let content_uri = format!("book:{}", book_id);
        let tag_ids: Vec<String> = sqlx::query_scalar(
            "SELECT tag_id FROM content_teaches WHERE content_uri = $1",
        )
        .bind(&content_uri)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

        for tag_id in &tag_ids {
            let _ = skill_service::light_skill(&state.pool, &user.did, tag_id, "mastered").await;
        }
    }

    Ok(StatusCode::OK)
}

// --- Remove reading status ---

pub async fn remove_reading_status(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Auth(user): Auth,
) -> ApiResult<StatusCode> {
    book_service::remove_reading_status(&state.pool, &book_id, &user.did).await?;
    Ok(StatusCode::OK)
}

// ---- Chapters ----

pub async fn list_chapters(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
) -> ApiResult<Json<Vec<book_service::BookChapter>>> {
    let chapters = book_service::list_chapters(&state.pool, &book_id).await?;
    Ok(Json(chapters))
}

pub async fn create_chapter(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Auth(user): Auth,
    Json(input): Json<book_service::CreateChapter>,
) -> ApiResult<Json<book_service::BookChapter>> {
    let id = format!("ch-{}", tid());
    let ch = book_service::create_chapter(&state.pool, &id, &book_id, &user.did, &input).await?;
    Ok(Json(ch))
}

#[derive(serde::Deserialize)]
pub struct UpdateChapterTagsInput {
    pub chapter_id: String,
    pub teaches: Vec<String>,
    pub prereqs: Vec<book_service::ChapterPrereq>,
}

pub async fn update_chapter_tags(
    State(state): State<AppState>,
    Path(_book_id): Path<String>,
    Auth(user): Auth,
    Json(input): Json<UpdateChapterTagsInput>,
) -> ApiResult<StatusCode> {
    book_service::set_chapter_tags(&state.pool, &input.chapter_id, &user.did, &input.teaches, &input.prereqs).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct DeleteChapterInput {
    pub chapter_id: String,
}

pub async fn delete_chapter(
    State(state): State<AppState>,
    Path(_book_id): Path<String>,
    Auth(_user): Auth,
    Json(input): Json<DeleteChapterInput>,
) -> ApiResult<StatusCode> {
    book_service::delete_chapter(&state.pool, &input.chapter_id).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct ChapterProgressInput {
    pub chapter_id: String,
    pub completed: bool,
}

pub async fn set_chapter_progress(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Auth(user): Auth,
    Json(input): Json<ChapterProgressInput>,
) -> ApiResult<StatusCode> {
    book_service::set_chapter_progress(&state.pool, &book_id, &input.chapter_id, &user.did, input.completed).await?;

    // Auto-learn: when chapter completed, light up chapter's teaches tags
    if input.completed {
        let content_uri = format!("chapter:{}", input.chapter_id);
        let tag_ids: Vec<String> = sqlx::query_scalar(
            "SELECT tag_id FROM content_teaches WHERE content_uri = $1",
        )
        .bind(&content_uri)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

        for tag_id in &tag_ids {
            let _ = skill_service::light_skill(&state.pool, &user.did, tag_id, "mastered").await;
        }
    }

    Ok(StatusCode::OK)
}

// --- Book cover: serve & upload ---

const COVER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const MAX_COVER_SIZE: usize = 5 * 1024 * 1024; // 5 MB

/// Serve a book cover image from local storage.
pub async fn get_cover(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response<Body> {
    let safe_id: String = id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    let covers_dir = state.data_dir.join("book-covers");
    for ext in COVER_EXTENSIONS {
        let path = covers_dir.join(format!("{safe_id}.{ext}"));
        if path.exists() {
            let content_type = match *ext {
                "png" => "image/png",
                "webp" => "image/webp",
                _ => "image/jpeg",
            };
            match tokio::fs::read(&path).await {
                Ok(data) => {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::CACHE_CONTROL, "public, max-age=86400")
                        .body(Body::from(data))
                        .unwrap();
                }
                Err(_) => break,
            }
        }
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap()
}

/// Upload a book cover image (multipart: field "file").
pub async fn upload_cover(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(_user): WriteAuth,
    mut multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError(fx_core::Error::BadRequest(format!("Multipart error: {e}")))
    })? {
        if field.name() == Some("file") {
            file_name = field.file_name().map(|s| s.to_string());
            file_data = Some(
                field.bytes().await
                    .map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?
                    .to_vec()
            );
        }
    }

    let data = file_data.ok_or(AppError(fx_core::Error::BadRequest("Missing file".into())))?;
    let name = file_name.unwrap_or_default();

    if data.len() > MAX_COVER_SIZE {
        return Err(AppError(fx_core::Error::BadRequest("Cover too large (max 5MB)".into())));
    }

    let ext = std::path::Path::new(&name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| "jpg".into());

    if !COVER_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest("Unsupported format. Use jpg, png, or webp.".into())));
    }

    let safe_id: String = id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    let dest = state.data_dir.join("book-covers").join(format!("{safe_id}.{ext}"));
    tokio::fs::write(&dest, &data).await?;

    let cover_url = format!("/api/book-covers/{safe_id}");
    sqlx::query("UPDATE books SET cover_url = $1 WHERE id = $2")
        .bind(&cover_url)
        .bind(&id)
        .execute(&state.pool)
        .await?;

    Ok(Json(serde_json::json!({ "cover_url": cover_url })))
}
