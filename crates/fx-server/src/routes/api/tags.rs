use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::models::*;
use fx_core::services::tag_service;
use fx_core::validation;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use super::{Auth, IdQuery};

#[derive(serde::Deserialize)]
pub struct ListTagsQuery {
    pub limit: Option<i64>,
}

pub async fn list_tags(
    State(state): State<AppState>,
    Query(q): Query<ListTagsQuery>,
) -> ApiResult<Json<Vec<Tag>>> {
    let limit = q.limit.unwrap_or(500).clamp(1, 1000);
    let tags = tag_service::list_tags(&state.pool, limit).await?;
    Ok(Json(tags))
}

pub async fn get_tag(
    State(state): State<AppState>,
    Query(IdQuery { id }): Query<IdQuery>,
) -> ApiResult<Json<Tag>> {
    let tag = tag_service::get_tag(&state.pool, &id).await?;
    Ok(Json(tag))
}

pub async fn create_tag(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<CreateTag>,
) -> ApiResult<(StatusCode, Json<Tag>)> {
    let mut errors = Vec::new();
    if let Err(e) = validation::validate_tag_id(&input.id) {
        errors.push(e);
    }
    if input.name.is_empty() || input.name.len() > 255 {
        errors.push(validation::ValidationError {
            field: "name".into(),
            message: "tag name must be 1-255 characters".into(),
        });
    }
    if !errors.is_empty() {
        return Err(AppError(fx_core::Error::Validation(errors)));
    }

    let tag = tag_service::create_tag(&state.pool, &input, &user.did).await?;
    Ok((StatusCode::CREATED, Json(tag)))
}
