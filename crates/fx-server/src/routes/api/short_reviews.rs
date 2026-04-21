//! Short-review endpoints for books and book series.
//!
//! A short review is a brief body of text (≤500 chars) attached to a book or
//! series. The rating is independent (`/books/{id}/rate`,
//! `/book-series/{id}/rate`) — the UI composes rating + short-review display
//! at read time, but they're written separately.

use axum::{Json, extract::{Path, Query, State}, http::StatusCode};
use fx_core::services::short_review_service;
use serde::Deserialize;

use crate::auth::{Auth, MaybeAuth, WriteAuth, pds_put_record, pds_delete_record};
use crate::error::ApiResult;
use crate::state::AppState;
use fx_core::util::now_rfc3339;

#[derive(Deserialize)]
pub struct Paging {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn clamp_limit(q: &Paging) -> (i64, i64) {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    (limit, offset)
}

fn book_short_review_uri(did: &str, book_id: &str) -> String {
    format!("at://{did}/{}/{book_id}", fx_atproto::lexicon::BOOK_SHORT_REVIEW)
}

fn series_short_review_uri(did: &str, series_id: &str) -> String {
    format!("at://{did}/{}/{series_id}", fx_atproto::lexicon::BOOK_SERIES_SHORT_REVIEW)
}

// ============== Book short reviews ============================================

pub async fn list_book_short_reviews(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    MaybeAuth(viewer): MaybeAuth,
    Query(q): Query<Paging>,
) -> ApiResult<Json<Vec<short_review_service::BookShortReview>>> {
    let (limit, offset) = clamp_limit(&q);
    let did = viewer.as_ref().map(|v| v.did.as_str());
    let rows = short_review_service::list_book_short_reviews(&state.pool, &book_id, did, limit, offset).await?;
    Ok(Json(rows))
}

pub async fn get_my_book_short_review(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Auth(user): Auth,
) -> ApiResult<Json<Option<short_review_service::BookShortReview>>> {
    let row = short_review_service::get_my_book_short_review(&state.pool, &user.did, &book_id).await?;
    Ok(Json(row))
}

pub async fn upsert_book_short_review(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<short_review_service::CreateBookShortReview>,
) -> ApiResult<(StatusCode, Json<short_review_service::BookShortReview>)> {
    // Confirm the book exists (surfaces NotFound cleanly rather than FK error).
    let _ = fx_core::services::book_service::get_book(&state.pool, &book_id).await?;

    let at_uri = book_short_review_uri(&user.did, &book_id);
    let row = short_review_service::upsert_book_short_review(
        &state.pool, &at_uri, &user.did, &book_id, &input,
    ).await?;

    let now = now_rfc3339();
    let mut record = serde_json::json!({
        "$type": fx_atproto::lexicon::BOOK_SHORT_REVIEW,
        "bookId": book_id,
        "body": input.body,
        "visibility": row.visibility,
        "createdAt": now,
        "updatedAt": now,
    });
    if let Some(edition) = &input.edition_id {
        record["editionId"] = serde_json::Value::String(edition.clone());
    }
    if let Some(lang) = &input.lang {
        record["lang"] = serde_json::Value::String(lang.clone());
    }
    pds_put_record(
        &state, &user.token,
        fx_atproto::lexicon::BOOK_SHORT_REVIEW,
        book_id.clone(), record, "book short review",
    ).await;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn delete_my_book_short_review(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    WriteAuth(user): WriteAuth,
) -> ApiResult<StatusCode> {
    short_review_service::delete_book_short_review(&state.pool, &user.did, &book_id).await?;
    pds_delete_record(
        &state, &user.token,
        fx_atproto::lexicon::BOOK_SHORT_REVIEW,
        book_id, "book short review delete",
    ).await;
    Ok(StatusCode::NO_CONTENT)
}

// ============== Series short reviews ==========================================

pub async fn list_series_short_reviews(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
    MaybeAuth(viewer): MaybeAuth,
    Query(q): Query<Paging>,
) -> ApiResult<Json<Vec<short_review_service::BookSeriesShortReview>>> {
    let (limit, offset) = clamp_limit(&q);
    let did = viewer.as_ref().map(|v| v.did.as_str());
    let rows = short_review_service::list_series_short_reviews(&state.pool, &series_id, did, limit, offset).await?;
    Ok(Json(rows))
}

pub async fn get_my_series_short_review(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
    Auth(user): Auth,
) -> ApiResult<Json<Option<short_review_service::BookSeriesShortReview>>> {
    let row = short_review_service::get_my_series_short_review(&state.pool, &user.did, &series_id).await?;
    Ok(Json(row))
}

pub async fn upsert_series_short_review(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<short_review_service::CreateSeriesShortReview>,
) -> ApiResult<(StatusCode, Json<short_review_service::BookSeriesShortReview>)> {
    let _ = fx_core::services::book_series_service::get_series(&state.pool, &series_id).await?;

    let at_uri = series_short_review_uri(&user.did, &series_id);
    let row = short_review_service::upsert_series_short_review(
        &state.pool, &at_uri, &user.did, &series_id, &input,
    ).await?;

    let now = now_rfc3339();
    let mut record = serde_json::json!({
        "$type": fx_atproto::lexicon::BOOK_SERIES_SHORT_REVIEW,
        "seriesId": series_id,
        "body": input.body,
        "visibility": row.visibility,
        "createdAt": now,
        "updatedAt": now,
    });
    if let Some(lang) = &input.lang {
        record["lang"] = serde_json::Value::String(lang.clone());
    }
    pds_put_record(
        &state, &user.token,
        fx_atproto::lexicon::BOOK_SERIES_SHORT_REVIEW,
        series_id.clone(), record, "series short review",
    ).await;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn delete_my_series_short_review(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
    WriteAuth(user): WriteAuth,
) -> ApiResult<StatusCode> {
    short_review_service::delete_series_short_review(&state.pool, &user.did, &series_id).await?;
    pds_delete_record(
        &state, &user.token,
        fx_atproto::lexicon::BOOK_SERIES_SHORT_REVIEW,
        series_id, "series short review delete",
    ).await;
    Ok(StatusCode::NO_CONTENT)
}
