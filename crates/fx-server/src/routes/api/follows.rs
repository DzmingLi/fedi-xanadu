use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::{social_service, notification_service};

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{Auth, WriteAuth};
use fx_core::util::tid;
use super::DidQuery;

#[derive(serde::Deserialize)]
pub struct FollowInput {
    did: String,
}

#[utoipa::path(get, path = "/api/v1/follows", responses((status = 200)), security(("bearer" = [])))]
pub async fn list_follows(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<social_service::FollowedUser>>> {
    let rows = social_service::list_follows(&state.pool, &user.did).await?;
    Ok(Json(rows))
}

#[utoipa::path(post, path = "/api/v1/follows", responses((status = 200)), security(("bearer" = [])))]
pub async fn follow(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<FollowInput>,
) -> ApiResult<StatusCode> {
    social_service::follow(&state.pool, &user.did, &input.did).await?;

    if let Err(e) = notification_service::create_notification(
        &state.pool, &tid(), &input.did, &user.did,
        "new_follower", None, None,
    ).await {
        tracing::warn!("notification failed: {e}");
    }

    Ok(StatusCode::OK)
}

#[utoipa::path(delete, path = "/api/v1/follows/remove", responses((status = 200)), security(("bearer" = [])))]
pub async fn unfollow(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<FollowInput>,
) -> ApiResult<StatusCode> {
    social_service::unfollow(&state.pool, &user.did, &input.did).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(post, path = "/api/v1/follows/seen", responses((status = 200)), security(("bearer" = [])))]
pub async fn mark_seen(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<FollowInput>,
) -> ApiResult<StatusCode> {
    social_service::mark_seen(&state.pool, &user.did, &input.did).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(get, path = "/api/v1/follows/following", params(("did" = String, Query)), responses((status = 200)))]
pub async fn following_by_did(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<Vec<social_service::FollowEntry>>> {
    let rows = social_service::following_by_did(&state.pool, &did).await?;
    Ok(Json(rows))
}

#[utoipa::path(get, path = "/api/v1/follows/followers", params(("did" = String, Query)), responses((status = 200)))]
pub async fn followers_by_did(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<Vec<social_service::FollowEntry>>> {
    let rows = social_service::followers_by_did(&state.pool, &did).await?;
    Ok(Json(rows))
}
