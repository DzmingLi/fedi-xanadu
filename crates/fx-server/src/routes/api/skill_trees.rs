use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::skill_tree_service;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::{WriteAuth, MaybeAuth};
use fx_core::util::tid;
use super::UriQuery;

#[derive(serde::Deserialize)]
pub struct ListSkillTreesQuery {
    pub limit: Option<i64>,
}

pub async fn list_skill_trees(
    State(state): State<AppState>,
    Query(q): Query<ListSkillTreesQuery>,
) -> ApiResult<Json<Vec<skill_tree_service::SkillTreeListRow>>> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let rows = skill_tree_service::list_skill_trees(&state.pool, limit).await?;
    Ok(Json(rows))
}

#[derive(serde::Deserialize)]
pub struct CreateSkillTreeInput {
    title: String,
    description: Option<String>,
    tag_id: Option<String>,
    edges: Vec<EdgeInput>,
}

#[derive(serde::Deserialize)]
pub(crate) struct EdgeInput {
    parent_tag: String,
    child_tag: String,
}

pub async fn create_skill_tree(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateSkillTreeInput>,
) -> ApiResult<(StatusCode, Json<skill_tree_service::SkillTreeRow>)> {
    if let Err(e) = fx_core::validation::validate_title(&input.title) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    let at_uri = format!("at://{}/li.dzming.fedi-xanadu.skilltree/{}", user.did, tid());

    let svc_input = skill_tree_service::CreateSkillTree {
        title: input.title,
        description: input.description,
        tag_id: input.tag_id,
        edges: input.edges.into_iter().map(|e| (e.parent_tag, e.child_tag)).collect(),
    };

    let row = skill_tree_service::create_skill_tree(&state.pool, &at_uri, &user.did, &svc_input).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn get_skill_tree_detail(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<skill_tree_service::SkillTreeDetailResponse>> {
    let detail = skill_tree_service::get_skill_tree_detail(&state.pool, &uri).await?;
    Ok(Json(detail))
}

#[derive(serde::Deserialize)]
pub(crate) struct ForkSkillTreeInput {
    uri: String,
}

pub async fn fork_skill_tree(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ForkSkillTreeInput>,
) -> ApiResult<(StatusCode, Json<skill_tree_service::SkillTreeRow>)> {
    let new_uri = format!("at://{}/li.dzming.fedi-xanadu.skilltree/{}", user.did, tid());
    let row = skill_tree_service::fork_skill_tree(&state.pool, &input.uri, &new_uri, &user.did).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

#[derive(serde::Deserialize)]
pub(crate) struct SkillTreeEdgeInput {
    tree_uri: String,
    parent_tag: String,
    child_tag: String,
}

pub async fn add_skill_tree_edge(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<SkillTreeEdgeInput>,
) -> ApiResult<StatusCode> {
    skill_tree_service::add_edge(&state.pool, &input.tree_uri, &user.did, &input.parent_tag, &input.child_tag).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_skill_tree_edge(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<SkillTreeEdgeInput>,
) -> ApiResult<StatusCode> {
    skill_tree_service::remove_edge(&state.pool, &input.tree_uri, &user.did, &input.parent_tag, &input.child_tag).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub(crate) struct AdoptTreeInput {
    tree_uri: String,
}

pub async fn adopt_skill_tree(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<AdoptTreeInput>,
) -> ApiResult<StatusCode> {
    skill_tree_service::adopt_skill_tree(&state.pool, &user.did, &input.tree_uri).await?;
    Ok(StatusCode::OK)
}

pub async fn get_active_tree(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
) -> ApiResult<Json<Option<skill_tree_service::SkillTreeDetailResponse>>> {
    let Some(user) = user else {
        return Ok(Json(None));
    };
    let result = skill_tree_service::get_active_tree(&state.pool, &user.did).await?;
    Ok(Json(result))
}
