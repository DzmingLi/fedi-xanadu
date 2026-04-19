use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, Response, header},
};
use fx_core::services::{author_service, book_service, skill_service, tag_service};

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
    crate::auth::MaybeAuth(user): crate::auth::MaybeAuth,
    Query(q): Query<ListBooksQuery>,
) -> ApiResult<Json<Vec<book_service::BookListItem>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let viewer = user.as_ref().map(|u| u.did.as_str());
    let books = book_service::list_books_rich(&state.pool, viewer, limit, offset).await?;
    Ok(Json(books))
}

// --- Get book detail ---

#[derive(serde::Serialize)]
pub struct BookDetail {
    pub book: book_service::Book,
    pub linked_authors: Vec<fx_core::services::author_service::Author>,
    pub editions: Vec<book_service::BookEdition>,
    pub chapters: Vec<book_service::BookChapterWithTags>,
    pub reviews: Vec<fx_core::models::Article>,
    pub notes: Vec<fx_core::models::Article>,
    pub review_count: usize,
    pub rating: book_service::BookRatingStats,
    pub my_rating: Option<i16>,
    pub my_reading_status: Option<book_service::ReadingStatus>,
    pub my_chapter_progress: Vec<book_service::ChapterProgress>,
    pub tags: Vec<String>,
    pub prereqs: Vec<String>,
    /// Derived from `tags` through the viewer's skill tree — ancestor tags
    /// that this book can be said to "belong to". Not stored; computed per
    /// request because it depends on the viewer's tree.
    pub topics: Vec<String>,
}

pub async fn get_book(
    State(state): State<AppState>,
    Path(id): Path<String>,
    MaybeAuth(user): MaybeAuth,
) -> ApiResult<Json<BookDetail>> {
    let book = if let Some(ref u) = user {
        book_service::get_book_for_viewer(&state.pool, &id, &u.did).await?
    } else {
        book_service::get_book(&state.pool, &id).await?
    };
    let linked_authors = author_service::list_book_authors(&state.pool, &id).await?;
    let editions = book_service::list_editions(&state.pool, &id).await?;
    let chapters = book_service::list_chapters_with_tags(&state.pool, &id).await?;
    let reviews = book_service::get_book_reviews(&state.pool, &id, 100, 0).await?;
    let notes = book_service::get_book_notes(&state.pool, &id, 100, 0).await?;
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
    let topics = tag_service::derive_topics(&state.pool, &content_uri)
        .await
        .unwrap_or_default();
    Ok(Json(BookDetail { book, linked_authors, editions, chapters, reviews, notes, review_count, rating, my_rating, my_reading_status, my_chapter_progress, tags, prereqs, topics }))
}

// --- Create book ---

pub async fn create_book(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<book_service::CreateBook>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if input.title.values().all(|v| v.trim().is_empty()) {
        return Err(AppError(fx_core::Error::BadRequest("title required".into())));
    }

    // Check for duplicate: same title (any language) + same authors
    let title_json = serde_json::to_value(&input.title)?;
    let dup: Option<String> = sqlx::query_scalar(
        "SELECT id FROM books WHERE authors = $1 AND EXISTS ( \
           SELECT 1 FROM jsonb_each_text(title) t1, jsonb_each_text($2::jsonb) t2 \
           WHERE t1.value = t2.value AND t1.value != '' \
         )"
    )
    .bind(&input.authors)
    .bind(&title_json)
    .fetch_optional(&state.pool)
    .await?;

    let id = format!("bk-{}", tid());
    let book = book_service::create_book(&state.pool, &id, &input, &user.did).await?;

    let mut resp = serde_json::to_value(&book)?;
    if let Some(existing_id) = dup {
        resp["warning"] = serde_json::json!(format!(
            "A book with the same title and authors already exists: {existing_id}"
        ));
    }

    Ok((StatusCode::CREATED, Json(resp)))
}

// --- Update book ---

#[derive(serde::Deserialize)]
pub struct UpdateBookInput {
    #[serde(default)]
    pub id: String,
    pub title: Option<std::collections::HashMap<String, String>>,
    pub subtitle: Option<std::collections::HashMap<String, String>>,
    pub description: Option<std::collections::HashMap<String, String>>,
    /// Short citation form ("CLRS", "SICP", "LADR"). Not translated.
    /// Pass empty string to clear; omit to leave unchanged.
    pub abbreviation: Option<String>,
    /// Ordered author names. When present, replaces both `books.authors`
    /// and the `book_authors` join table.
    pub authors: Option<Vec<String>>,
    /// Tag ids that this book teaches. When present, replaces
    /// content_teaches for this book.
    pub tags: Option<Vec<String>>,
    /// Tag ids that this book requires as prereqs. When present, replaces
    /// content_prereqs for this book (all marked as 'required').
    pub prereqs: Option<Vec<String>>,
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
        "subtitle": old.subtitle,
        "description": old.description,
        "abbreviation": old.abbreviation,
    });

    if let Some(ref title) = input.title {
        let json = serde_json::to_value(title)?;
        sqlx::query("UPDATE books SET title = $1 WHERE id = $2")
            .bind(&json).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref subtitle) = input.subtitle {
        let json = serde_json::to_value(subtitle)?;
        sqlx::query("UPDATE books SET subtitle = $1 WHERE id = $2")
            .bind(&json).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref desc) = input.description {
        let json = serde_json::to_value(desc)?;
        sqlx::query("UPDATE books SET description = $1 WHERE id = $2")
            .bind(&json).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref abbr) = input.abbreviation {
        let trimmed = abbr.trim();
        let value: Option<&str> = if trimmed.is_empty() { None } else { Some(trimmed) };
        sqlx::query("UPDATE books SET abbreviation = $1 WHERE id = $2")
            .bind(value).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref authors) = input.authors {
        let cleaned: Vec<String> = authors.iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        sqlx::query("UPDATE books SET authors = $1 WHERE id = $2")
            .bind(&cleaned).bind(&input.id).execute(&state.pool).await?;
        sqlx::query("DELETE FROM book_authors WHERE book_id = $1")
            .bind(&input.id).execute(&state.pool).await?;
        for (position, name) in cleaned.iter().enumerate() {
            let author_id = author_service::get_or_create_author(&state.pool, name).await?;
            author_service::link_author_to_book(&state.pool, &input.id, &author_id, position as i16).await?;
        }
    }
    if let Some(ref tags) = input.tags {
        let content_uri = format!("book:{}", input.id);
        sqlx::query("DELETE FROM content_teaches WHERE content_uri = $1")
            .bind(&content_uri).execute(&state.pool).await?;
        for tag_id in tags {
            let t = tag_id.trim();
            if t.is_empty() { continue; }
            sqlx::query(
                "INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
            ).bind(t).bind(t).bind(&user.did).execute(&state.pool).await?;
            sqlx::query(
                "INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            ).bind(&content_uri).bind(t).execute(&state.pool).await?;
        }
    }
    if let Some(ref prereqs) = input.prereqs {
        let content_uri = format!("book:{}", input.id);
        sqlx::query("DELETE FROM content_prereqs WHERE content_uri = $1")
            .bind(&content_uri).execute(&state.pool).await?;
        for tag_id in prereqs {
            let t = tag_id.trim();
            if t.is_empty() { continue; }
            sqlx::query(
                "INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
            ).bind(t).bind(t).bind(&user.did).execute(&state.pool).await?;
            sqlx::query(
                "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) VALUES ($1, $2, 'required') ON CONFLICT DO NOTHING",
            ).bind(&content_uri).bind(t).execute(&state.pool).await?;
        }
    }
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
        "abbreviation": input.abbreviation,
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

pub async fn update_edition(
    State(state): State<AppState>,
    Path((book_id, edition_id)): Path<(String, String)>,
    WriteAuth(_user): WriteAuth,
    Json(input): Json<book_service::CreateEdition>,
) -> ApiResult<Json<book_service::BookEdition>> {
    let _ = book_service::get_book(&state.pool, &book_id).await?;
    let edition = book_service::update_edition(&state.pool, &edition_id, &input).await?;
    Ok(Json(edition))
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

pub async fn unrate_book(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Auth(user): Auth,
) -> ApiResult<Json<book_service::BookRatingStats>> {
    book_service::unrate_book(&state.pool, &book_id, &user.did).await?;
    let stats = book_service::get_rating_stats(&state.pool, &book_id).await?;
    Ok(Json(stats))
}

// --- Set reading status ---

#[derive(serde::Deserialize)]
pub struct SetReadingStatusInput {
    pub status: String,
    #[serde(default)]
    pub progress: i16,
    pub edition_id: Option<String>,
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

    // Set preferred edition if specified
    if let Some(ref eid) = input.edition_id {
        sqlx::query("UPDATE book_reading_status SET preferred_edition_id = $1 WHERE book_id = $2 AND user_did = $3")
            .bind(eid).bind(&book_id).bind(&user.did).execute(&state.pool).await?;
    }

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

    Ok(StatusCode::NO_CONTENT)
}

// --- Remove reading status ---

pub async fn remove_reading_status(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Auth(user): Auth,
) -> ApiResult<StatusCode> {
    book_service::remove_reading_status(&state.pool, &book_id, &user.did).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Preferred edition cover ---

#[derive(serde::Deserialize)]
pub struct SetPreferredEditionInput {
    pub edition_id: Option<String>,
}

pub async fn set_preferred_edition(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Auth(user): Auth,
    Json(input): Json<SetPreferredEditionInput>,
) -> ApiResult<StatusCode> {
    // Upsert into book_reading_status to store preference (create row if needed)
    sqlx::query(
        "INSERT INTO book_reading_status (book_id, user_did, status, preferred_edition_id) \
         VALUES ($1, $2, 'want_to_read', $3) \
         ON CONFLICT (book_id, user_did) DO UPDATE SET preferred_edition_id = $3",
    )
    .bind(&book_id)
    .bind(&user.did)
    .bind(&input.edition_id)
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
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
    Ok(StatusCode::NO_CONTENT)
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
    Ok(StatusCode::NO_CONTENT)
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

    Ok(StatusCode::NO_CONTENT)
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

/// Upload a cover for a specific edition.
pub async fn upload_edition_cover(
    State(state): State<AppState>,
    Path((book_id, edition_id)): Path<(String, String)>,
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
            file_data = Some(field.bytes().await
                .map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?.to_vec());
        }
    }

    let data = file_data.ok_or(AppError(fx_core::Error::BadRequest("Missing file".into())))?;
    if data.len() > MAX_COVER_SIZE {
        return Err(AppError(fx_core::Error::BadRequest("Cover too large (max 5MB)".into())));
    }

    let ext = std::path::Path::new(file_name.as_deref().unwrap_or(""))
        .extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_else(|| "jpg".into());
    if !COVER_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest("Use jpg, png, or webp".into())));
    }

    let safe_id: String = edition_id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();

    let dest = state.data_dir.join("book-covers").join(format!("{safe_id}.{ext}"));
    tokio::fs::write(&dest, &data).await?;

    let cover_url = format!("/api/book-covers/{safe_id}");
    sqlx::query("UPDATE book_editions SET cover_url = $1 WHERE id = $2 AND book_id = $3")
        .bind(&cover_url).bind(&edition_id).bind(&book_id).execute(&state.pool).await?;

    Ok(Json(serde_json::json!({ "cover_url": cover_url })))
}

// --- Book resources (supplementary materials) ---

pub async fn list_resources(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
) -> ApiResult<Json<Vec<book_service::BookResource>>> {
    let resources = book_service::list_book_resources(&state.pool, &book_id).await?;
    Ok(Json(resources))
}

#[derive(serde::Deserialize)]
pub struct AddResourceInput {
    pub edition_id: Option<String>,
    pub kind: String,
    pub label: String,
    pub url: String,
    #[serde(default)]
    pub position: i16,
}

pub async fn add_resource(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(book_id): Path<String>,
    Json(input): Json<AddResourceInput>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let id = book_service::add_book_resource(
        &state.pool, &book_id, input.edition_id.as_deref(),
        &input.kind, &input.label, &input.url, input.position, &user.did,
    ).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

pub async fn delete_resource(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_book_id, resource_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    book_service::delete_book_resource(&state.pool, &resource_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
