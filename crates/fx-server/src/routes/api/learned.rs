use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::learned_service;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{Auth, WriteAuth, MaybeAuth};
use super::UriQuery;

#[derive(serde::Deserialize)]
pub struct MarkLearnedInput {
    article_uri: String,
}

#[utoipa::path(post, path = "/api/v1/learned", responses((status = 200)), security(("bearer" = [])))]
pub async fn mark_learned(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<MarkLearnedInput>,
) -> ApiResult<StatusCode> {
    learned_service::mark_learned(&state.pool, &user.did, &input.article_uri).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(delete, path = "/api/v1/learned/remove", responses((status = 200)), security(("bearer" = [])))]
pub async fn unmark_learned(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<MarkLearnedInput>,
) -> ApiResult<StatusCode> {
    learned_service::unmark_learned(&state.pool, &user.did, &input.article_uri).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(get, path = "/api/v1/learned/check", params(("uri" = String, Query)), responses((status = 200)))]
pub async fn is_learned(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let learned = if let Some(u) = user {
        learned_service::is_learned(&state.pool, &u.did, &uri).await?
    } else {
        false
    };
    Ok(Json(serde_json::json!({ "learned": learned })))
}

#[utoipa::path(get, path = "/api/v1/learned", responses((status = 200)), security(("bearer" = [])))]
pub async fn list_learned(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<learned_service::LearnedMark>>> {
    let marks = learned_service::list_learned(&state.pool, &user.did).await?;
    Ok(Json(marks))
}
