use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::notification_service;

use crate::error::ApiResult;
use crate::state::AppState;
use super::Auth;

#[derive(serde::Deserialize)]
pub struct ListNotificationsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_notifications(
    State(state): State<AppState>,
    Auth(user): Auth,
    Query(q): Query<ListNotificationsQuery>,
) -> ApiResult<Json<Vec<notification_service::Notification>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let rows = notification_service::list_notifications(&state.pool, &user.did, limit, offset).await?;
    Ok(Json(rows))
}

pub async fn unread_count(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<serde_json::Value>> {
    let count = notification_service::unread_count(&state.pool, &user.did).await?;
    Ok(Json(serde_json::json!({ "count": count })))
}

#[derive(serde::Deserialize)]
pub struct MarkReadInput {
    pub id: String,
}

pub async fn mark_read(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<MarkReadInput>,
) -> ApiResult<StatusCode> {
    notification_service::mark_read(&state.pool, &user.did, &input.id).await?;
    Ok(StatusCode::OK)
}

pub async fn mark_all_read(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<StatusCode> {
    notification_service::mark_all_read(&state.pool, &user.did).await?;
    Ok(StatusCode::OK)
}
