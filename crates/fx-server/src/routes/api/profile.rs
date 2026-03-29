use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::social_service;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::WriteAuth;
use super::DidQuery;

#[derive(serde::Deserialize)]
pub(crate) struct UpdateProfileLinksInput {
    links: Vec<social_service::ProfileLink>,
}

#[utoipa::path(get, path = "/api/v1/profile", params(("did" = String, Query)), responses((status = 200)))]
pub async fn get_profile(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<social_service::ProfileResponse>> {
    let profile = social_service::get_profile(&state.pool, &did).await?;
    Ok(Json(profile))
}

#[utoipa::path(put, path = "/api/v1/profile/links", responses((status = 200)), security(("bearer" = [])))]
pub async fn update_profile_links(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UpdateProfileLinksInput>,
) -> ApiResult<StatusCode> {
    let links_json = serde_json::to_string(&input.links)?;
    social_service::update_profile_links(&state.pool, &user.did, &links_json).await?;
    Ok(StatusCode::OK)
}
