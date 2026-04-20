use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::skill_tree_service;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::{WriteAuth, MaybeAuth, pds_put_record, pds_delete_record};
use fx_core::util::{tid, now_rfc3339};
use super::UriQuery;

/// Rebuild the full PDS record for a skill tree from current DB state and
/// put it under the owner's repo. Called after create/fork and any edge or
/// prereq mutation so external consumers see a consistent snapshot.
async fn publish_skill_tree(state: &AppState, token: &str, tree_uri: &str) {
    let Ok(tree) = skill_tree_service::get_skill_tree(&state.pool, tree_uri).await else {
        return;
    };
    let Some(rkey) = tree_uri.rsplit('/').next().map(str::to_string) else { return; };

    let edges: Vec<(String, String)> = sqlx::query_as(
        "SELECT parent_tag, child_tag FROM skill_tree_edges WHERE tree_uri = $1",
    )
    .bind(tree_uri)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();
    let prereqs: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT from_tag, to_tag, prereq_type FROM skill_tree_prereqs WHERE tree_uri = $1",
    )
    .bind(tree_uri)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let mut record = serde_json::json!({
        "$type": fx_atproto::lexicon::SKILL_TREE,
        "title": tree.title,
        "edges": edges.iter().map(|(p, c)| serde_json::json!({"parent": p, "child": c})).collect::<Vec<_>>(),
        "prereqs": prereqs.iter().map(|(f, t, pt)| serde_json::json!({"from": f, "to": t, "prereqType": pt})).collect::<Vec<_>>(),
        "createdAt": tree.created_at,
    });
    if let Some(d) = tree.description { record["description"] = serde_json::Value::String(d); }
    if let Some(t) = tree.tag_id      { record["tagId"]       = serde_json::Value::String(t); }
    if let Some(f) = tree.forked_from { record["forkedFrom"]  = serde_json::Value::String(f); }

    pds_put_record(state, token, fx_atproto::lexicon::SKILL_TREE, rkey, record, "publish skill tree").await;
}

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
    #[serde(default)]
    prereqs: Vec<PrereqInput>,
}

#[derive(serde::Deserialize)]
pub(crate) struct EdgeInput {
    parent_tag: String,
    child_tag: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct PrereqInput {
    from_tag: String,
    to_tag: String,
    #[serde(default = "default_prereq_type")]
    prereq_type: String,
}

fn default_prereq_type() -> String { "required".to_string() }

pub async fn create_skill_tree(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateSkillTreeInput>,
) -> ApiResult<(StatusCode, Json<skill_tree_service::SkillTreeRow>)> {
    if let Err(e) = fx_core::validation::validate_title(&input.title) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    let at_uri = format!("at://{}/at.nightbo.skilltree/{}", user.did, tid());

    let svc_input = skill_tree_service::CreateSkillTree {
        title: input.title,
        description: input.description,
        tag_id: input.tag_id,
        edges: input.edges.into_iter().map(|e| (e.parent_tag, e.child_tag)).collect(),
        prereqs: input.prereqs.into_iter().map(|p| (p.from_tag, p.to_tag, p.prereq_type)).collect(),
    };

    let row = skill_tree_service::create_skill_tree(&state.pool, &at_uri, &user.did, &svc_input).await?;
    publish_skill_tree(&state, &user.token, &at_uri).await;
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
    let new_uri = format!("at://{}/at.nightbo.skilltree/{}", user.did, tid());
    let row = skill_tree_service::fork_skill_tree(&state.pool, &input.uri, &new_uri, &user.did).await?;
    publish_skill_tree(&state, &user.token, &new_uri).await;
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
    publish_skill_tree(&state, &user.token, &input.tree_uri).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_skill_tree_edge(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<SkillTreeEdgeInput>,
) -> ApiResult<StatusCode> {
    skill_tree_service::remove_edge(&state.pool, &input.tree_uri, &user.did, &input.parent_tag, &input.child_tag).await?;
    publish_skill_tree(&state, &user.token, &input.tree_uri).await;
    Ok(StatusCode::NO_CONTENT)
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
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub(crate) struct SkillTreePrereqInput {
    tree_uri: String,
    from_tag: String,
    to_tag: String,
    #[serde(default = "default_prereq_type")]
    prereq_type: String,
}

pub async fn add_skill_tree_prereq(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<SkillTreePrereqInput>,
) -> ApiResult<StatusCode> {
    skill_tree_service::add_prereq(&state.pool, &input.tree_uri, &user.did, &input.from_tag, &input.to_tag, &input.prereq_type).await?;
    publish_skill_tree(&state, &user.token, &input.tree_uri).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_skill_tree_prereq(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<SkillTreePrereqInput>,
) -> ApiResult<StatusCode> {
    skill_tree_service::remove_prereq(&state.pool, &input.tree_uri, &user.did, &input.from_tag, &input.to_tag).await?;
    publish_skill_tree(&state, &user.token, &input.tree_uri).await;
    Ok(StatusCode::NO_CONTENT)
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
