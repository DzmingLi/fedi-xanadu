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

pub async fn get_profile(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<social_service::ProfileResponse>> {
    let profile = social_service::get_profile(&state.pool, &did).await?;
    Ok(Json(profile))
}

pub async fn update_profile_links(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UpdateProfileLinksInput>,
) -> ApiResult<StatusCode> {
    let links_json = serde_json::to_string(&input.links)?;
    social_service::update_profile_links(&state.pool, &user.did, &links_json).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub(crate) struct UpdateBioInput {
    bio: String,
}

pub async fn update_bio(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UpdateBioInput>,
) -> ApiResult<StatusCode> {
    sqlx::query("UPDATE profiles SET bio = $1 WHERE did = $2")
        .bind(&input.bio)
        .bind(&user.did)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::OK)
}

pub async fn update_publications(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<Vec<social_service::PublicationEntry>>,
) -> ApiResult<StatusCode> {
    let json = serde_json::to_value(&input)?;
    sqlx::query("UPDATE profiles SET publications = $1 WHERE did = $2")
        .bind(&json).bind(&user.did).execute(&state.pool).await?;
    Ok(StatusCode::OK)
}

pub async fn update_projects(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<Vec<social_service::ProjectEntry>>,
) -> ApiResult<StatusCode> {
    let json = serde_json::to_value(&input)?;
    sqlx::query("UPDATE profiles SET projects = $1 WHERE did = $2")
        .bind(&json).bind(&user.did).execute(&state.pool).await?;
    Ok(StatusCode::OK)
}

pub async fn update_teaching(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<Vec<social_service::TeachingEntry>>,
) -> ApiResult<StatusCode> {
    let json = serde_json::to_value(&input)?;
    sqlx::query("UPDATE profiles SET teaching = $1 WHERE did = $2")
        .bind(&json).bind(&user.did).execute(&state.pool).await?;
    Ok(StatusCode::OK)
}

pub async fn get_user_listings(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<Vec<fx_core::services::listing_service::Listing>>> {
    let listings = fx_core::services::listing_service::list_my_listings(&state.pool, &did).await?;
    let open: Vec<_> = listings.into_iter().filter(|l| l.is_open).collect();
    Ok(Json(open))
}
