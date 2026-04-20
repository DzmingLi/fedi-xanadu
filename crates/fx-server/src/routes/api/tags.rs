use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fx_core::models::*;
use fx_core::services::tag_service;
use fx_core::validation;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::WriteAuth;

#[derive(serde::Deserialize)]
pub struct SearchTagsQuery {
    pub q: String,
    pub limit: Option<i64>,
}

pub async fn search_tags(
    State(state): State<AppState>,
    Query(q): Query<SearchTagsQuery>,
) -> ApiResult<Json<Vec<Tag>>> {
    let limit = q.limit.unwrap_or(10).clamp(1, 50);
    let tags = tag_service::search_tags(&state.pool, &q.q, limit).await?;
    Ok(Json(tags))
}

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
    Path(id): Path<String>,
) -> ApiResult<Json<Tag>> {
    let tag = tag_service::get_tag(&state.pool, &id).await?;
    Ok(Json(tag))
}

#[derive(serde::Serialize)]
pub struct GroupView {
    pub members: Vec<Tag>,
    /// Map of lang → member id. Admin picks one rep per language; the
    /// frontend shows ★ next to each lang's rep and the UI picks its
    /// display label with `representatives[uiLocale] ?? representatives.en`.
    pub representatives: std::collections::HashMap<String, String>,
}

/// List every sibling tag in the alias/translation group that `id` belongs
/// to, plus the per-language representatives.
pub async fn list_group_siblings(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<GroupView>> {
    let tags = tag_service::list_group_siblings(&state.pool, &id).await?;
    let representatives = tag_service::list_group_representatives(&state.pool, &id).await?;
    Ok(Json(GroupView { members: tags, representatives }))
}

#[derive(serde::Deserialize)]
pub struct AddGroupMemberInput {
    /// Slug / id for the new tag. Must be unique across all tags.
    pub id: String,
    /// Display name; usually equals id for non-English tags.
    pub name: String,
    /// ISO locale code for this tag (e.g. "zh", "ja", "fr").
    pub lang: String,
}

/// Add a sibling tag to the same group as `id`. Used when admin wants to
/// add a translation or an alias for an existing tag.
pub async fn add_group_member(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(user): crate::auth::Auth,
    Json(input): Json<AddGroupMemberInput>,
) -> ApiResult<Json<Tag>> {
    let tag = tag_service::add_group_member(
        &state.pool, &id, &input.id, &input.name, &input.lang, &user.did,
    ).await?;
    Ok(Json(tag))
}

/// Remove a tag from its group — the tag row is deleted. If the tag is the
/// last remaining member of its group, the group row is also removed.
pub async fn remove_group_member(
    State(state): State<AppState>,
    Path((_group_anchor, member_id)): Path<(String, String)>,
    crate::auth::Auth(_user): crate::auth::Auth,
) -> ApiResult<Json<serde_json::Value>> {
    tag_service::remove_group_member(&state.pool, &member_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(serde::Deserialize)]
pub struct RepresentativeInput { pub member_id: String }

/// Mark one of the group's members as the representative — the single
/// label the UI picks for display when it needs one (prereqs, mastery
/// badges, teach tags, etc.).
pub async fn set_group_representative(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(_user): crate::auth::Auth,
    Json(input): Json<RepresentativeInput>,
) -> ApiResult<Json<serde_json::Value>> {
    tag_service::set_group_representative(&state.pool, &id, &input.member_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

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

#[derive(serde::Deserialize)]
pub struct UpdateTagNamesInput {
    pub names: HashMap<String, String>,
}

pub async fn update_tag_names(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(_user): crate::auth::Auth,
    Json(input): Json<UpdateTagNamesInput>,
) -> ApiResult<Json<Tag>> {
    let tag = tag_service::update_tag_names(&state.pool, &id, &input.names).await?;
    Ok(Json(tag))
}

// Aliases — global, any logged-in user can add/remove.

#[derive(serde::Deserialize)]
pub struct AliasInput {
    pub alias: String,
}

pub async fn list_aliases(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<String>>> {
    let aliases = tag_service::list_aliases(&state.pool, &id).await?;
    Ok(Json(aliases))
}

pub async fn add_alias(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(_user): crate::auth::Auth,
    Json(input): Json<AliasInput>,
) -> ApiResult<Json<serde_json::Value>> {
    tag_service::add_alias(&state.pool, input.alias.trim(), &id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn remove_alias(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(_user): crate::auth::Auth,
    Json(input): Json<AliasInput>,
) -> ApiResult<Json<serde_json::Value>> {
    // id is accepted in the path for RESTful symmetry but the alias is the
    // unique key — tag_id association lives alongside it.
    let _ = id;
    tag_service::remove_alias(&state.pool, input.alias.trim()).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// --- Set content teaches ---

#[derive(serde::Deserialize)]
pub struct SetTeachInput {
    pub content_uri: String,
    pub tag_id: String,
}

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
    Ok(StatusCode::NO_CONTENT)
}
