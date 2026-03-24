use axum::{
    Json,
    extract::State,
};
use fx_core::models::*;

use crate::error::ApiResult;
use crate::state::AppState;
use super::AuthDid;

#[derive(serde::Serialize)]
pub(crate) struct GraphNode {
    id: String,
    name: String,
    lit: bool,
}

#[derive(serde::Serialize)]
pub(crate) struct GraphEdge {
    from: String,
    to: String,
    #[serde(rename = "type")]
    edge_type: String,
}

#[derive(serde::Serialize)]
pub(crate) struct GraphData {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

pub async fn get_graph(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
) -> ApiResult<Json<GraphData>> {
    let tags = sqlx::query_as::<_, Tag>("SELECT id, name, description, created_by, created_at FROM tags ORDER BY name")
        .fetch_all(&state.pool)
        .await?;

    let skills: Vec<String> = sqlx::query_scalar(
        "SELECT tag_id FROM user_skills WHERE did = ?",
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    #[derive(sqlx::FromRow)]
    struct PrereqEdge {
        article_uri: String,
        tag_id: String,
        prereq_type: String,
    }

    let prereq_edges = sqlx::query_as::<_, PrereqEdge>(
        "SELECT DISTINCT ap.article_uri, ap.tag_id, ap.prereq_type FROM article_prereqs ap",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    #[derive(sqlx::FromRow)]
    struct ATag {
        article_uri: String,
        tag_id: String,
    }

    let article_tags = sqlx::query_as::<_, ATag>(
        "SELECT article_uri, tag_id FROM article_tags",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let mut edges = Vec::new();
    for prereq in &prereq_edges {
        for at in &article_tags {
            if at.article_uri == prereq.article_uri && prereq.tag_id != at.tag_id {
                edges.push(GraphEdge {
                    from: prereq.tag_id.clone(),
                    to: at.tag_id.clone(),
                    edge_type: prereq.prereq_type.clone(),
                });
            }
        }
    }

    let nodes = tags
        .iter()
        .map(|t| GraphNode {
            id: t.id.clone(),
            name: t.name.clone(),
            lit: skills.contains(&t.id),
        })
        .collect();

    Ok(Json(GraphData { nodes, edges }))
}
