use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fx_core::services::ad_service::{self, Ad, AdSlot, CreateAd, UpdateAd};

use crate::auth::AdminAuth;
use crate::error::ApiResult;
use crate::state::AppState;
use fx_core::util::tid;

#[derive(serde::Deserialize)]
pub struct ServeQuery {
    placement: Option<String>,
}

// ---------------------------------------------------------------------------
// Public
// ---------------------------------------------------------------------------

/// GET /ads/serve?placement=sidebar — return one ad for display
pub async fn serve(
    State(state): State<AppState>,
    Query(q): Query<ServeQuery>,
) -> ApiResult<Json<Option<AdSlot>>> {
    let placement = q.placement.as_deref().unwrap_or("sidebar");
    let ad = ad_service::serve(&state.pool, placement).await?;
    Ok(Json(ad))
}

/// POST /ads/:id/click — record a click
pub async fn click(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    ad_service::record_click(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Admin
// ---------------------------------------------------------------------------

/// GET /admin/ads — list all ads
pub async fn list_all(
    State(state): State<AppState>,
    _admin: AdminAuth,
) -> ApiResult<Json<Vec<Ad>>> {
    let ads = ad_service::list_all(&state.pool).await?;
    Ok(Json(ads))
}

/// POST /admin/ads — create an ad
pub async fn create(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<CreateAd>,
) -> ApiResult<(StatusCode, Json<Ad>)> {
    let id = format!("ad-{}", tid());
    let ad = ad_service::create(&state.pool, &id, &input).await?;
    Ok((StatusCode::CREATED, Json(ad)))
}

/// PUT /admin/ads/:id — update an ad
pub async fn update(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Path(id): Path<String>,
    Json(input): Json<UpdateAd>,
) -> ApiResult<Json<Ad>> {
    let ad = ad_service::update(&state.pool, &id, &input).await?;
    Ok(Json(ad))
}

/// DELETE /admin/ads/:id — delete an ad
pub async fn delete(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    ad_service::delete(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
