use axum::{Json, extract::State};
use fx_core::services::block_service;

use crate::error::ApiResult;
use crate::state::AppState;
use super::{Auth, WriteAuth};

#[derive(serde::Deserialize)]
pub struct BlockInput {
    pub did: String,
}

pub async fn block_user(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<BlockInput>,
) -> ApiResult<Json<serde_json::Value>> {
    block_service::block_user(&state.pool, &user.did, &input.did).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn unblock_user(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<BlockInput>,
) -> ApiResult<Json<serde_json::Value>> {
    block_service::unblock_user(&state.pool, &user.did, &input.did).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn list_blocked_users(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<block_service::BlockedUser>>> {
    let list = block_service::list_blocked_users(&state.pool, &user.did).await?;
    Ok(Json(list))
}

pub async fn list_blocked_dids(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<String>>> {
    let dids = block_service::list_blocked_dids(&state.pool, &user.did).await?;
    Ok(Json(dids))
}
