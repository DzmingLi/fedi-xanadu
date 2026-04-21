use axum::{Json, extract::State};
use fx_core::services::tag_hierarchy_service;

use crate::auth::Auth;
use crate::error::ApiResult;
use crate::state::AppState;

pub async fn list_tag_parents(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<tag_hierarchy_service::TagParent>>> {
    let rows = tag_hierarchy_service::list_all(&state.pool).await?;
    Ok(Json(rows))
}

#[derive(serde::Deserialize)]
pub struct EdgeInput {
    pub parent_tag: String,
    pub child_tag: String,
}

pub async fn add_tag_parent(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<EdgeInput>,
) -> ApiResult<Json<serde_json::Value>> {
    let parent = input.parent_tag.trim();
    let child = input.child_tag.trim();
    fx_core::services::tag_service::require_tag_id(parent)?;
    fx_core::services::tag_service::require_tag_id(child)?;
    tag_hierarchy_service::add_edge(&state.pool, parent, child, &user.did).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn remove_tag_parent(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<EdgeInput>,
) -> ApiResult<Json<serde_json::Value>> {
    let parent = input.parent_tag.trim();
    let child = input.child_tag.trim();
    fx_core::services::tag_service::require_tag_id(parent)?;
    fx_core::services::tag_service::require_tag_id(child)?;
    tag_hierarchy_service::remove_edge(&state.pool, parent, child, &user.did).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
