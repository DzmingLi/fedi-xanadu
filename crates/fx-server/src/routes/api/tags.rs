use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use fx_core::models::*;
use fx_core::services::tag_service;
use fx_core::validation;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::WriteAuth;

#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct SearchTagsQuery {
    pub q: String,
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/tags/search",
    params(SearchTagsQuery),
    responses((status = 200, description = "Matching tags", body = Vec<Tag>))
)]
pub async fn search_tags(
    State(state): State<AppState>,
    Query(q): Query<SearchTagsQuery>,
) -> ApiResult<Json<Vec<Tag>>> {
    let limit = q.limit.unwrap_or(10).clamp(1, 50);
    let tags = tag_service::search_tags(&state.pool, &q.q, limit).await?;
    Ok(Json(tags))
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct ListTagsQuery {
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/tags",
    params(ListTagsQuery),
    responses((status = 200, description = "All tags", body = Vec<Tag>))
)]
pub async fn list_tags(
    State(state): State<AppState>,
    Query(q): Query<ListTagsQuery>,
) -> ApiResult<Json<Vec<Tag>>> {
    let limit = q.limit.unwrap_or(500).clamp(1, 1000);
    let tags = tag_service::list_tags(&state.pool, limit).await?;
    Ok(Json(tags))
}

#[utoipa::path(
    get,
    path = "/api/v1/tags/{id}",
    params(("id" = String, Path, description = "Tag ID")),
    responses(
        (status = 200, description = "Tag details", body = Tag),
        (status = 404, description = "Not found"),
    )
)]
pub async fn get_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Tag>> {
    let tag = tag_service::get_tag(&state.pool, &id).await?;
    Ok(Json(tag))
}

#[utoipa::path(
    post,
    path = "/api/v1/tags",
    request_body = CreateTag,
    responses(
        (status = 201, description = "Tag created", body = Tag),
        (status = 422, description = "Validation error"),
    ),
    security(("bearer" = []))
)]
pub async fn create_tag(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
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

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateTagNamesInput {
    pub names: HashMap<String, String>,
}

#[utoipa::path(
    put,
    path = "/api/v1/tags/{id}/names",
    params(("id" = String, Path, description = "Tag ID")),
    request_body = UpdateTagNamesInput,
    responses((status = 200, description = "Tag updated", body = Tag)),
    security(("bearer" = []))
)]
pub async fn update_tag_names(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(input): Json<UpdateTagNamesInput>,
) -> ApiResult<Json<Tag>> {
    // Admin-only endpoint
    let secret = state.admin_secret.as_deref()
        .ok_or(AppError(fx_core::Error::Forbidden { action: "admin not configured" }))?;
    let provided = headers.get("x-admin-secret")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError(fx_core::Error::Unauthorized))?;
    if provided != secret {
        return Err(AppError(fx_core::Error::Unauthorized));
    }

    let tag = tag_service::update_tag_names(&state.pool, &id, &input.names).await?;
    Ok(Json(tag))
}

// --- Set content teaches ---

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct SetTeachInput {
    pub content_uri: String,
    pub tag_id: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/tags/teach",
    request_body = SetTeachInput,
    responses((status = 200, description = "Teach association set")),
    security(("bearer" = []))
)]
pub async fn set_teach(
    State(state): State<AppState>,
    crate::auth::Auth(_user): crate::auth::Auth,
    Json(input): Json<SetTeachInput>,
) -> ApiResult<StatusCode> {
    // Ensure tag exists
    tag_service::ensure_tag(&state.pool, &input.tag_id, &_user.did).await?;
    sqlx::query(
        "INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(&input.content_uri)
    .bind(&input.tag_id)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))?;
    Ok(StatusCode::OK)
}
