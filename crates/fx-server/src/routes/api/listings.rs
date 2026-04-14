use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fx_core::services::listing_service::{self, CreateListing, Listing};

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{Auth, WriteAuth};
use fx_core::util::tid;

#[derive(serde::Deserialize)]
pub struct ListListingsQuery {
    kind: Option<String>,
    tag: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

pub async fn list_listings(
    State(state): State<AppState>,
    Query(q): Query<ListListingsQuery>,
) -> ApiResult<Json<Vec<Listing>>> {
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    let offset = q.offset.unwrap_or(0).max(0);
    let listings = listing_service::list_listings(
        &state.pool,
        q.kind.as_deref(),
        q.tag.as_deref(),
        limit,
        offset,
    ).await?;
    Ok(Json(listings))
}

pub async fn get_listing(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Listing>> {
    let listing = listing_service::get_listing(&state.pool, &id).await?;
    Ok(Json(listing))
}

pub async fn create_listing(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateListing>,
) -> ApiResult<(StatusCode, Json<Listing>)> {
    let id = format!("lst-{}", tid());
    let listing = listing_service::create_listing(&state.pool, &id, &user.did, &input).await?;
    Ok((StatusCode::CREATED, Json(listing)))
}

pub async fn update_listing(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<CreateListing>,
) -> ApiResult<Json<Listing>> {
    let listing = listing_service::update_listing(&state.pool, &id, &user.did, &input).await?;
    Ok(Json(listing))
}

pub async fn close_listing(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    listing_service::close_listing(&state.pool, &id, &user.did).await?;
    Ok(StatusCode::OK)
}

pub async fn reopen_listing(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    listing_service::reopen_listing(&state.pool, &id, &user.did).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_listing(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    listing_service::delete_listing(&state.pool, &id, &user.did).await?;
    Ok(StatusCode::OK)
}

pub async fn my_listings(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<Listing>>> {
    let listings = listing_service::list_my_listings(&state.pool, &user.did).await?;
    Ok(Json(listings))
}

pub async fn matched_listings(
    State(state): State<AppState>,
    Auth(user): Auth,
    Query(q): Query<ListListingsQuery>,
) -> ApiResult<Json<Vec<Listing>>> {
    let limit = q.limit.unwrap_or(20).clamp(1, 50);
    let listings = listing_service::match_for_user(&state.pool, &user.did, limit).await?;
    Ok(Json(listings))
}
