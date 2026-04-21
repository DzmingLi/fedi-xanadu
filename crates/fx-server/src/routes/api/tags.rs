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

/// Input-boundary endpoint for the edit UI: take a label or brand-new
/// name (what the user typed or picked) and return the canonical
/// `tag_id`. Creates a new label + tag row if the name is new.
/// Forbids `tg-…` input — callers that already have a tag_id don't
/// need this endpoint.
#[derive(serde::Deserialize)]
pub struct ResolveTagInput {
    pub input: String,
}

#[derive(serde::Serialize)]
pub struct ResolveTagOutput {
    pub tag_id: String,
}

pub async fn resolve_tag(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ResolveTagInput>,
) -> ApiResult<Json<ResolveTagOutput>> {
    let trimmed = input.input.trim();
    if trimmed.is_empty() {
        return Err(AppError(fx_core::Error::BadRequest("input must not be empty".into())));
    }
    let mut conn = state.pool.acquire().await.map_err(|e| fx_core::Error::Internal(e.to_string()))?;
    let tag_id = tag_service::resolve_tag_id(&mut conn, trimmed, &user.did).await?;
    Ok(Json(ResolveTagOutput { tag_id }))
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

#[derive(serde::Serialize)]
pub struct TagWithMeta {
    #[serde(flatten)]
    pub tag: Tag,
    pub pending_deletion: bool,
}

pub async fn get_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<TagWithMeta>> {
    let tag = tag_service::get_tag(&state.pool, &id).await?;
    // Deletion requests key on tag_id; resolve whatever caller passed.
    let pending_deletion = tag_service::has_pending_deletion(&state.pool, &tag.tag_id).await.unwrap_or(false);
    Ok(Json(TagWithMeta { tag, pending_deletion }))
}

#[derive(serde::Serialize)]
pub struct GroupView {
    /// Every name row attached to the concept.
    pub members: Vec<Tag>,
}

/// List every name attached to the same concept as `id`. `id` accepts
/// either a name id (`tn-…`) or a tag id (`tg-…`).
pub async fn list_group_siblings(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<GroupView>> {
    let members = tag_service::list_group_siblings(&state.pool, &id).await?;
    Ok(Json(GroupView { members }))
}

#[derive(serde::Deserialize)]
pub struct AddGroupMemberInput {
    /// The name string in this language.
    pub name: String,
    /// ISO locale code for this name (e.g. "zh", "ja", "fr").
    pub lang: String,
}

/// Add a new name to the concept that `id` belongs to. `id` accepts a
/// name id or a tag id; the new name gets a fresh `tn-…` id.
pub async fn add_group_member(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(user): crate::auth::Auth,
    Json(input): Json<AddGroupMemberInput>,
) -> ApiResult<Json<Tag>> {
    let tag_id = tag_service::lookup_tag_id(&state.pool, &id).await?
        .ok_or_else(|| AppError(fx_core::Error::NotFound { entity: "tag", id: id.clone() }))?;
    let tag = tag_service::add_name(&state.pool, &tag_id, &input.name, &input.lang, &user.did).await?;
    Ok(Json(tag))
}

/// Remove a single name from its concept. If the concept has no names
/// left, drop the concept and everything that referenced it.
pub async fn remove_group_member(
    State(state): State<AppState>,
    Path((_anchor, member_id)): Path<(String, String)>,
    crate::auth::Auth(user): crate::auth::Auth,
) -> ApiResult<Json<serde_json::Value>> {
    tag_service::remove_name(&state.pool, &member_id, &user.did).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(serde::Deserialize, Default)]
pub struct DeletionRequestInput {
    #[serde(default)]
    pub reason: String,
}

/// User-initiated tag deletion request. An admin must later approve;
/// reason is optional. Fails if the tag is already under review.
pub async fn request_tag_deletion(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(user): crate::auth::Auth,
    Json(input): Json<DeletionRequestInput>,
) -> ApiResult<Json<tag_service::TagDeletionRequest>> {
    let req = tag_service::request_tag_deletion(&state.pool, &id, &user.did, input.reason.trim()).await?;
    Ok(Json(req))
}

#[derive(serde::Deserialize)]
pub struct MergeInput {
    /// The tag whose names should move into `id`. Accepts name_id or tag_id.
    pub from: String,
}

/// Merge another concept's names into the concept that `id` refers to.
/// The other concept's name rows move over, then it's deleted.
pub async fn merge_groups(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(user): crate::auth::Auth,
    Json(input): Json<MergeInput>,
) -> ApiResult<Json<serde_json::Value>> {
    tag_service::merge_tag(&state.pool, &input.from, &id, &user.did).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Set the viewer's preferred name for the concept. `id` accepts
/// name_id or tag_id (resolved to the concept); body picks the name
/// row that should be shown to this viewer from now on.
#[derive(serde::Deserialize)]
pub struct SetPrefInput { pub name_id: String }

pub async fn set_user_name_pref(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(user): crate::auth::Auth,
    Json(input): Json<SetPrefInput>,
) -> ApiResult<Json<serde_json::Value>> {
    let tag_id = tag_service::lookup_tag_id(&state.pool, &id).await?
        .ok_or_else(|| AppError(fx_core::Error::NotFound { entity: "tag", id: id.clone() }))?;
    tag_service::set_user_name_pref(&state.pool, &user.did, &tag_id, &input.name_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn clear_user_name_pref(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(user): crate::auth::Auth,
) -> ApiResult<Json<serde_json::Value>> {
    let tag_id = tag_service::lookup_tag_id(&state.pool, &id).await?
        .ok_or_else(|| AppError(fx_core::Error::NotFound { entity: "tag", id: id.clone() }))?;
    tag_service::clear_user_name_pref(&state.pool, &user.did, &tag_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// List viewer's name preferences: `{tag_id → name_id}`. Used by the
/// frontend tagStore to overlay user pref on top of the default display.
pub async fn list_my_name_prefs(
    State(state): State<AppState>,
    crate::auth::Auth(user): crate::auth::Auth,
) -> ApiResult<Json<HashMap<String, String>>> {
    let prefs = tag_service::list_user_name_prefs(&state.pool, &user.did).await?;
    Ok(Json(prefs))
}

pub async fn create_tag(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateTag>,
) -> ApiResult<(StatusCode, Json<Tag>)> {
    if input.name.is_empty() || input.name.len() > 255 {
        return Err(AppError(fx_core::Error::Validation(vec![validation::ValidationError {
            field: "name".into(),
            message: "tag name must be 1-255 characters".into(),
        }])));
    }
    let tag = tag_service::create_tag(&state.pool, &input, &user.did).await?;
    Ok((StatusCode::CREATED, Json(tag)))
}

#[derive(serde::Deserialize)]
pub struct UpdateTagNamesInput {
    pub names: HashMap<String, String>,
}

pub async fn get_tag_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<tag_service::TagAuditEntry>>> {
    let tag_id = tag_service::lookup_tag_id(&state.pool, &id).await?
        .ok_or_else(|| AppError(fx_core::Error::NotFound { entity: "tag", id: id.clone() }))?;
    let entries = tag_service::list_tag_audit(&state.pool, &tag_id, 200).await?;
    Ok(Json(entries))
}

pub async fn update_tag_names(
    State(state): State<AppState>,
    Path(id): Path<String>,
    crate::auth::Auth(user): crate::auth::Auth,
    Json(input): Json<UpdateTagNamesInput>,
) -> ApiResult<Json<Tag>> {
    let tag = tag_service::update_tag_names(&state.pool, &id, &input.names, &user.did).await?;
    Ok(Json(tag))
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
    tag_service::require_tag_id(&input.tag_id)?;
    sqlx::query(
        "INSERT INTO content_teaches (content_uri, tag_id) \
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(&input.content_uri)
    .bind(&input.tag_id)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))?;
    Ok(StatusCode::NO_CONTENT)
}
