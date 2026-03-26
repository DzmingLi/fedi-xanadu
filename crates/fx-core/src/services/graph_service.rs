use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use crate::models::Tag;

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub names: HashMap<String, String>,
    pub lit: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub edge_type: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Build the knowledge graph using a SQL JOIN instead of O(n²) Rust loop.
pub async fn build_knowledge_graph(pool: &PgPool, did: Option<&str>) -> crate::Result<GraphData> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name, names, description, created_by, created_at FROM tags ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    let skills: HashSet<String> = if let Some(did) = did {
        sqlx::query_scalar("SELECT tag_id FROM user_skills WHERE did = $1")
            .bind(did)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect()
    } else {
        HashSet::new()
    };

    // Compute edges via SQL JOIN instead of O(n²) nested loop
    #[derive(sqlx::FromRow)]
    struct EdgeRow {
        from_tag: String,
        to_tag: String,
        prereq_type: String,
    }

    let edge_rows = sqlx::query_as::<_, EdgeRow>(
        "SELECT DISTINCT ap.tag_id AS from_tag, at2.tag_id AS to_tag, ap.prereq_type \
         FROM article_prereqs ap \
         JOIN article_tags at2 ON at2.article_uri = ap.article_uri AND at2.tag_id != ap.tag_id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Deduplicate edges (same from→to with same type)
    let mut seen = HashSet::new();
    let edges: Vec<GraphEdge> = edge_rows
        .into_iter()
        .filter(|e| seen.insert((e.from_tag.clone(), e.to_tag.clone(), e.prereq_type.clone())))
        .map(|e| GraphEdge {
            from: e.from_tag,
            to: e.to_tag,
            edge_type: e.prereq_type,
        })
        .collect();

    let nodes = tags
        .iter()
        .map(|t| GraphNode {
            id: t.id.clone(),
            name: t.name.clone(),
            names: t.names.0.clone(),
            lit: skills.contains(&t.id),
        })
        .collect();

    Ok(GraphData { nodes, edges })
}
