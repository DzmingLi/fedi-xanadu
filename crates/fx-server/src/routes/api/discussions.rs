use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fx_core::services::{article_service, discussion_service};
use fx_core::util::{tid, uri_to_node_id, content_hash};

use crate::error::{AppError, ApiResult, require_owner};
use crate::state::AppState;
use crate::auth::WriteAuth;

use super::articles::sync_meta_to_db;

// --- Create discussion ---

#[derive(serde::Deserialize)]
pub struct CreateDiscussionInput {
    pub target_uri: String,
    pub source_uri: String,
    pub title: String,
    pub body: Option<String>,
    pub change_hashes: Vec<String>,
}

pub async fn create_discussion(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateDiscussionInput>,
) -> ApiResult<(StatusCode, Json<discussion_service::Discussion>)> {
    let id = format!("disc-{}", tid());

    let disc = discussion_service::create_discussion(
        &state.pool, &id,
        &input.target_uri, &input.source_uri,
        &user.did, &input.title, input.body.as_deref(),
        &input.change_hashes,
    ).await?;

    Ok((StatusCode::CREATED, Json(disc)))
}

// --- List discussions for an article ---

#[derive(serde::Deserialize)]
pub struct DiscussionListQuery {
    pub uri: String,
}

pub async fn list_discussions(
    State(state): State<AppState>,
    Query(q): Query<DiscussionListQuery>,
) -> ApiResult<Json<Vec<discussion_service::Discussion>>> {
    let discussions = discussion_service::list_discussions(&state.pool, &q.uri).await?;
    Ok(Json(discussions))
}

// --- Get discussion detail ---

pub async fn get_discussion(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<discussion_service::DiscussionDetail>> {
    let detail = discussion_service::get_discussion(&state.pool, &id).await?;
    Ok(Json(detail))
}

// --- Update discussion status ---

#[derive(serde::Deserialize)]
pub struct UpdateStatusInput {
    pub status: String,
}

pub async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UpdateStatusInput>,
) -> ApiResult<StatusCode> {
    let disc = discussion_service::get_discussion(&state.pool, &id).await?;
    // Only target article owner or discussion author can update status
    let target_owner = article_service::get_article_owner(&state.pool, &disc.discussion.target_uri).await?;
    if user.did != target_owner && user.did != disc.discussion.author_did {
        return Err(AppError(fx_core::Error::Forbidden { action: "update discussion status" }));
    }

    discussion_service::update_status(&state.pool, &id, &input.status).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Apply a single change from discussion ---

#[derive(serde::Deserialize)]
pub struct ApplyChangeInput {
    pub change_hash: String,
}

pub async fn apply_discussion_change(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ApplyChangeInput>,
) -> ApiResult<Json<serde_json::Value>> {
    let disc = discussion_service::get_discussion(&state.pool, &id).await?;
    let target_owner = article_service::get_article_owner(&state.pool, &disc.discussion.target_uri).await?;
    require_owner(Some(&target_owner), &user.did)?;

    let source_node = uri_to_node_id(&disc.discussion.source_uri);
    let target_node = uri_to_node_id(&disc.discussion.target_uri);

    // Apply the change
    state.pijul.apply(&source_node, &target_node, &input.change_hash)
        .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;

    // Read updated content and check conflicts
    let src_ext = article_service::get_content_format(&state.pool, &disc.discussion.target_uri).await?;
    let ext = fx_render::format_extension(&src_ext);
    let content_bytes = state.pijul.get_file_content(&target_node, &format!("content.{ext}"))
        .map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))?;
    let content = String::from_utf8_lossy(&content_bytes).to_string();
    let has_conflicts = content.contains(">>>>>>>") || content.contains("<<<<<<<");

    // Re-render + record if no conflicts
    let repo_path = state.pijul.repo_path(&target_node);
    if !has_conflicts {
        if src_ext != "html" {
            if let Ok(rendered) = super::articles::render_content(&src_ext, &content, &repo_path) {
                let _ = tokio::fs::write(repo_path.join("content.html"), rendered).await;
            }
        }
        let message = format!("Applied change from discussion {id}");
        let _ = state.pijul.record(&target_node, &message, Some(&user.did));
        let hash = content_hash(&content);
        let _ = article_service::update_article_content_hash(&state.pool, &disc.discussion.target_uri, &hash).await;
        sync_meta_to_db(&state, &disc.discussion.target_uri, &repo_path).await;
    }

    // Mark change as applied
    discussion_service::mark_change_applied(&state.pool, &id, &input.change_hash).await?;

    // Auto-merge if all changes applied
    if discussion_service::all_changes_applied(&state.pool, &id).await? {
        discussion_service::update_status(&state.pool, &id, "merged").await?;
    }

    Ok(Json(serde_json::json!({ "has_conflicts": has_conflicts })))
}

// --- Apply all pending changes ---

pub async fn apply_all_discussion_changes(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
) -> ApiResult<Json<serde_json::Value>> {
    let disc = discussion_service::get_discussion(&state.pool, &id).await?;
    let target_owner = article_service::get_article_owner(&state.pool, &disc.discussion.target_uri).await?;
    require_owner(Some(&target_owner), &user.did)?;

    let source_node = uri_to_node_id(&disc.discussion.source_uri);
    let target_node = uri_to_node_id(&disc.discussion.target_uri);

    let pending: Vec<_> = disc.changes.iter().filter(|c| !c.applied).collect();

    for change in &pending {
        if let Err(e) = state.pijul.apply(&source_node, &target_node, &change.change_hash) {
            tracing::warn!("failed to apply change {}: {e}", change.change_hash);
            continue;
        }
        let _ = discussion_service::mark_change_applied(&state.pool, &id, &change.change_hash).await;
    }

    // Final state: read content, check conflicts, render, record
    let src_ext = article_service::get_content_format(&state.pool, &disc.discussion.target_uri).await?;
    let ext = fx_render::format_extension(&src_ext);
    let content_bytes = state.pijul.get_file_content(&target_node, &format!("content.{ext}"))
        .map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))?;
    let content = String::from_utf8_lossy(&content_bytes).to_string();
    let any_conflicts = content.contains(">>>>>>>") || content.contains("<<<<<<<");

    let repo_path = state.pijul.repo_path(&target_node);
    if !any_conflicts {
        if src_ext != "html" {
            if let Ok(rendered) = super::articles::render_content(&src_ext, &content, &repo_path) {
                let _ = tokio::fs::write(repo_path.join("content.html"), rendered).await;
            }
        }
        let message = format!("Applied all changes from discussion {id}");
        let _ = state.pijul.record(&target_node, &message, Some(&user.did));
        let hash = content_hash(&content);
        let _ = article_service::update_article_content_hash(&state.pool, &disc.discussion.target_uri, &hash).await;
        sync_meta_to_db(&state, &disc.discussion.target_uri, &repo_path).await;
    }

    // Auto-merge status
    if discussion_service::all_changes_applied(&state.pool, &id).await? {
        discussion_service::update_status(&state.pool, &id, "merged").await?;
    }

    Ok(Json(serde_json::json!({ "has_conflicts": any_conflicts, "applied": pending.len() })))
}
