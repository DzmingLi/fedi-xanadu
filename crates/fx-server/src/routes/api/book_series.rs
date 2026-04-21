//! HTTP endpoints for book series — CRUD, membership, ratings, edit log, cover.
//!
//! Short-review endpoints for series live in `short_reviews.rs`. Comments go
//! through the generic `api/comments.rs` with `content_uri = book_series:{id}`.

use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, Response, header},
};
use fx_core::services::{book_series_service, book_service, short_review_service};
use serde::Deserialize;

use crate::auth::{Auth, MaybeAuth, WriteAuth};
use crate::error::{AppError, ApiResult};
use crate::state::AppState;

const SERIES_COVER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif"];
const SERIES_MAX_COVER_SIZE: usize = 5 * 1024 * 1024;

// ---- List / detail ------------------------------------------------------

#[derive(Deserialize)]
pub struct ListSeriesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_series(
    State(state): State<AppState>,
    Query(q): Query<ListSeriesQuery>,
) -> ApiResult<Json<Vec<book_series_service::BookSeriesListItem>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let rows = book_series_service::list_series(&state.pool, limit, offset).await?;
    Ok(Json(rows))
}

#[derive(serde::Serialize)]
pub struct BookSeriesDetail {
    pub series: book_series_service::BookSeries,
    pub members: Vec<book_series_service::BookSeriesMemberItem>,
    pub member_rating: book_service::BookRatingStats,
    pub series_rating: book_service::BookRatingStats,
    pub my_series_rating: Option<i16>,
    pub my_short_review: Option<short_review_service::BookSeriesShortReview>,
    pub short_review_count: i64,
    pub recent_short_reviews: Vec<short_review_service::BookSeriesShortReview>,
}

pub async fn get_series(
    State(state): State<AppState>,
    Path(id): Path<String>,
    MaybeAuth(viewer): MaybeAuth,
) -> ApiResult<Json<BookSeriesDetail>> {
    let series = book_series_service::get_series(&state.pool, &id).await?;
    let members = book_series_service::list_members(&state.pool, &id).await?;
    let member_rating = book_series_service::get_member_rating_aggregate(&state.pool, &id).await?;
    let series_rating = book_series_service::get_series_rating_stats(&state.pool, &id).await?;

    let viewer_did = viewer.as_ref().map(|v| v.did.as_str());
    let my_series_rating = if let Some(d) = viewer_did {
        book_series_service::get_user_series_rating(&state.pool, &id, d).await?
    } else { None };
    let my_short_review = if let Some(d) = viewer_did {
        short_review_service::get_my_series_short_review(&state.pool, d, &id).await?
    } else { None };
    let short_review_count = short_review_service::count_series_short_reviews(&state.pool, &id).await?;
    let recent_short_reviews = short_review_service::list_series_short_reviews(
        &state.pool, &id, viewer_did, 5, 0,
    ).await?;

    Ok(Json(BookSeriesDetail {
        series, members, member_rating, series_rating,
        my_series_rating, my_short_review, short_review_count, recent_short_reviews,
    }))
}

// ---- Create / update / delete ------------------------------------------

pub async fn create_series(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<book_series_service::CreateBookSeries>,
) -> ApiResult<(StatusCode, Json<book_series_service::BookSeries>)> {
    if input.id.trim().is_empty() || !input.id.starts_with("bs-") {
        return Err(AppError(fx_core::Error::BadRequest(
            "series id must start with 'bs-' and be non-empty".into(),
        )));
    }
    if input.title.values().all(|v| v.trim().is_empty()) {
        return Err(AppError(fx_core::Error::BadRequest("title required".into())));
    }
    let series = book_series_service::create_series(&state.pool, &input, &user.did).await?;
    Ok((StatusCode::CREATED, Json(series)))
}

pub async fn update_series(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Auth(user): Auth,
    Json(input): Json<book_series_service::UpdateBookSeries>,
) -> ApiResult<Json<book_series_service::BookSeries>> {
    let series = book_series_service::update_series(&state.pool, &id, &input, &user.did).await?;
    Ok(Json(series))
}

pub async fn delete_series(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Auth(_user): Auth,
) -> ApiResult<StatusCode> {
    book_series_service::delete_series(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_series_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<book_series_service::BookSeriesEditLog>>> {
    let log = book_series_service::list_edit_history(&state.pool, &id).await?;
    Ok(Json(log))
}

// ---- Membership ---------------------------------------------------------

#[derive(Deserialize)]
pub struct AddMemberInput {
    pub book_id: String,
    #[serde(default)]
    pub position: Option<i16>,
}

pub async fn add_member(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
    Auth(_user): Auth,
    Json(input): Json<AddMemberInput>,
) -> ApiResult<StatusCode> {
    let _ = book_series_service::get_series(&state.pool, &series_id).await?;
    let _ = book_service::get_book(&state.pool, &input.book_id).await?;
    let position = input.position.unwrap_or(0);
    book_series_service::add_member(&state.pool, &series_id, &input.book_id, position).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_member(
    State(state): State<AppState>,
    Path((series_id, book_id)): Path<(String, String)>,
    Auth(_user): Auth,
) -> ApiResult<StatusCode> {
    book_series_service::remove_member(&state.pool, &series_id, &book_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ReorderMembersInput {
    pub orders: Vec<MemberOrder>,
}
#[derive(Deserialize)]
pub struct MemberOrder {
    pub book_id: String,
    pub position: i16,
}

pub async fn reorder_members(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
    Auth(_user): Auth,
    Json(input): Json<ReorderMembersInput>,
) -> ApiResult<StatusCode> {
    let orders: Vec<(String, i16)> = input.orders.into_iter()
        .map(|m| (m.book_id, m.position))
        .collect();
    book_series_service::reorder_members(&state.pool, &series_id, &orders).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- Ratings ------------------------------------------------------------

#[derive(Deserialize)]
pub struct RateSeriesInput {
    pub rating: i16,
}

pub async fn rate_series(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
    Auth(user): Auth,
    Json(input): Json<RateSeriesInput>,
) -> ApiResult<Json<book_service::BookRatingStats>> {
    if !(1..=10).contains(&input.rating) {
        return Err(AppError(fx_core::Error::BadRequest(
            "rating must be 1-10 (half-stars)".into(),
        )));
    }
    let _ = book_series_service::get_series(&state.pool, &series_id).await?;
    book_series_service::rate_series(&state.pool, &series_id, &user.did, input.rating).await?;

    let now = fx_core::util::now_rfc3339();
    let record = serde_json::json!({
        "$type": fx_atproto::lexicon::BOOK_SERIES_RATING,
        "seriesId": series_id,
        "rating": input.rating,
        "createdAt": now,
        "updatedAt": now,
    });
    crate::auth::pds_put_record(
        &state,
        match &user.token { t if !t.is_empty() => t, _ => "" },
        fx_atproto::lexicon::BOOK_SERIES_RATING,
        series_id.clone(), record, "series rate",
    ).await;

    let stats = book_series_service::get_series_rating_stats(&state.pool, &series_id).await?;
    Ok(Json(stats))
}

pub async fn unrate_series(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
    Auth(user): Auth,
) -> ApiResult<Json<book_service::BookRatingStats>> {
    book_series_service::unrate_series(&state.pool, &series_id, &user.did).await?;
    crate::auth::pds_delete_record(
        &state, &user.token,
        fx_atproto::lexicon::BOOK_SERIES_RATING,
        series_id.clone(), "series unrate",
    ).await;
    let stats = book_series_service::get_series_rating_stats(&state.pool, &series_id).await?;
    Ok(Json(stats))
}

// ---- Cover upload / serve ----------------------------------------------

pub async fn get_series_cover(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response<Body> {
    let safe_id: String = id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let covers_dir = state.data_dir.join("book-series-covers");
    for ext in SERIES_COVER_EXTENSIONS {
        let path = covers_dir.join(format!("{safe_id}.{ext}"));
        if path.exists() {
            let content_type = match *ext {
                "png" => "image/png",
                "webp" => "image/webp",
                "gif" => "image/gif",
                _ => "image/jpeg",
            };
            if let Ok(data) = tokio::fs::read(&path).await {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, content_type)
                    .header(header::CACHE_CONTROL, "public, max-age=86400")
                    .body(Body::from(data))
                    .unwrap();
            }
        }
    }
    Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()
}

pub async fn upload_series_cover(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
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
    if data.len() > SERIES_MAX_COVER_SIZE {
        return Err(AppError(fx_core::Error::BadRequest("Cover too large (max 5MB)".into())));
    }

    let ext = std::path::Path::new(file_name.as_deref().unwrap_or(""))
        .extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_else(|| "jpg".into());
    if !SERIES_COVER_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest("Use jpg, png, webp, or gif".into())));
    }

    let safe_id: String = series_id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();

    let covers_dir = state.data_dir.join("book-series-covers");
    tokio::fs::create_dir_all(&covers_dir).await?;
    let dest = covers_dir.join(format!("{safe_id}.{ext}"));
    tokio::fs::write(&dest, &data).await?;

    let cover_url = format!("/api/book-series-covers/{safe_id}");
    sqlx::query("UPDATE book_series SET cover_url = $1 WHERE id = $2")
        .bind(&cover_url).bind(&series_id).execute(&state.pool).await?;

    Ok(Json(serde_json::json!({ "cover_url": cover_url })))
}
