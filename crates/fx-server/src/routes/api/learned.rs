use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::learned_service;

use crate::error::ApiResult;
use crate::state::AppState;
use super::{Auth, MaybeAuth, UriQuery};

#[derive(serde::Deserialize)]
pub struct MarkLearnedInput {
    article_uri: String,
}

pub async fn mark_learned(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<MarkLearnedInput>,
) -> ApiResult<StatusCode> {
    learned_service::mark_learned(&state.pool, &user.did, &input.article_uri).await?;
    Ok(StatusCode::OK)
}

pub async fn unmark_learned(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<MarkLearnedInput>,
) -> ApiResult<StatusCode> {
    learned_service::unmark_learned(&state.pool, &user.did, &input.article_uri).await?;
    Ok(StatusCode::OK)
}

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

pub async fn list_learned(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<learned_service::LearnedMark>>> {
    let marks = learned_service::list_learned(&state.pool, &user.did).await?;
    Ok(Json(marks))
}
