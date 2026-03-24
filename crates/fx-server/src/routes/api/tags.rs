use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::models::*;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use super::{AuthDid, IdQuery};

pub async fn list_tags(State(state): State<AppState>) -> ApiResult<Json<Vec<Tag>>> {
    let tags = sqlx::query_as::<_, Tag>("SELECT id, name, description, created_by, created_at FROM tags ORDER BY name")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(tags))
}

pub async fn get_tag(
    State(state): State<AppState>,
    Query(IdQuery { id }): Query<IdQuery>,
) -> ApiResult<Json<Tag>> {
    let tag = sqlx::query_as::<_, Tag>("SELECT id, name, description, created_by, created_at FROM tags WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound("tag not found".into()))?;
    Ok(Json(tag))
}

pub async fn create_tag(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    Json(input): Json<CreateTag>,
) -> ApiResult<(StatusCode, Json<Tag>)> {
    sqlx::query("INSERT INTO tags (id, name, description, created_by) VALUES (?, ?, ?, ?)")
        .bind(&input.id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&did)
        .execute(&state.pool)
        .await?;

    let tag = sqlx::query_as::<_, Tag>("SELECT id, name, description, created_by, created_at FROM tags WHERE id = ?")
        .bind(&input.id)
        .fetch_one(&state.pool)
        .await?;

    Ok((StatusCode::CREATED, Json(tag)))
}
