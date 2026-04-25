use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use fx_core::services::paper_service::{
    self, CreatePaper, CreateVersion, Paper, PaperDetailResponse, PaperListItem, PaperVersion,
};
use fx_core::util::tid;
use serde::Deserialize;

use crate::auth::WriteAuth;
use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_papers(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<PaperListItem>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let rows = paper_service::list_papers(&state.pool, limit, offset).await?;
    Ok(Json(rows))
}

pub async fn get_paper(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<PaperDetailResponse>> {
    let detail = paper_service::get_paper_detail(&state.pool, &id).await?;
    Ok(Json(detail))
}

/// Create a paper. Any logged-in user can mint one — the gating is on the
/// "claim authorship" side (the author-verified flow already covers that),
/// not on adding a paper entry to the directory.
pub async fn create_paper(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreatePaper>,
) -> ApiResult<(StatusCode, Json<Paper>)> {
    let id = format!("pap-{}", tid());
    let paper = paper_service::create_paper(&state.pool, &id, &user.did, &input).await?;
    Ok((StatusCode::CREATED, Json(paper)))
}

pub async fn delete_paper(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    paper_service::delete_paper(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_version(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<CreateVersion>,
) -> ApiResult<(StatusCode, Json<PaperVersion>)> {
    let vid = format!("pv-{}", tid());
    let version = paper_service::add_version(&state.pool, &id, &vid, &input).await?;
    Ok((StatusCode::CREATED, Json(version)))
}

#[derive(Deserialize)]
pub struct DeleteVersionQuery {
    pub version_id: String,
}

pub async fn delete_version(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(_id): Path<String>,
    Query(q): Query<DeleteVersionQuery>,
) -> ApiResult<StatusCode> {
    paper_service::delete_version(&state.pool, &q.version_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct AddAuthorInput {
    pub author_id: String,
    #[serde(default)]
    pub position: i16,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String { "author".into() }

pub async fn add_author(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<AddAuthorInput>,
) -> ApiResult<StatusCode> {
    paper_service::add_author(&state.pool, &id, &input.author_id, input.position, &input.role).await?;
    Ok(StatusCode::NO_CONTENT)
}
