use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fx_core::services::{article_service, discussion_service};
use fx_core::util::tid;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::WriteAuth;

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

// NOTE: apply_discussion_change + apply_all_discussion_changes removed with
// the pijul-knot deprecation. Re-introducing them requires fx-pijul to grow
// cross-repo `apply` — tracked as follow-up work.
