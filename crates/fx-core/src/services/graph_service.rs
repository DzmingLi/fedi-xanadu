use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub names: HashMap<String, String>,
    pub lit: bool,
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub edge_type: String,
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Build the knowledge graph. Nodes are tags (concepts); each node's `id`
/// is the tag's canonical label id so front-end routing keeps working,
/// and `names` carries the per-language translation map.
pub async fn build_knowledge_graph(pool: &PgPool, did: Option<&str>) -> crate::Result<GraphData> {
    #[derive(sqlx::FromRow)]
    struct TagRow {
        tag_id: String,
        label_id: String,
        name: String,
        names: sqlx::types::Json<HashMap<String, String>>,
    }

    let tag_rows = sqlx::query_as::<_, TagRow>(
        "SELECT t.id AS tag_id, \
                tag_canonical_label(t.id) AS label_id, \
                (SELECT name FROM tag_labels WHERE id = tag_canonical_label(t.id)) AS name, \
                tag_label_map(t.id) AS names \
         FROM tags t \
         WHERE EXISTS (SELECT 1 FROM tag_labels l WHERE l.tag_id = t.id AND l.removed_at IS NULL) \
         ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    // Skills are stored at tag level. Build a HashSet of lit tag_ids.
    let lit_tags: HashSet<String> = if let Some(did) = did {
        sqlx::query_scalar(
            "SELECT DISTINCT tag_id FROM ( \
                 SELECT ct.tag_id FROM learned_marks lm \
                 JOIN content_teaches ct ON ct.content_uri = lm.article_uri \
                 WHERE lm.did = $1 \
                 UNION \
                 SELECT tag_id FROM user_skills WHERE did = $1 \
             ) AS combined",
        )
            .bind(did)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect()
    } else {
        HashSet::new()
    };

    // Map tag_id → canonical label id for edge endpoints.
    let tag_to_label: HashMap<String, String> = tag_rows
        .iter()
        .map(|r| (r.tag_id.clone(), r.label_id.clone()))
        .collect();

    // Edges: two tags X → Y when some content teaches Y and requires X.
    #[derive(sqlx::FromRow)]
    struct EdgeRow {
        from_tag: String,
        to_tag: String,
        prereq_type: String,
    }

    let edge_rows = sqlx::query_as::<_, EdgeRow>(
        "SELECT DISTINCT cp.tag_id AS from_tag, ct.tag_id AS to_tag, cp.prereq_type \
         FROM content_prereqs cp \
         JOIN content_teaches ct ON ct.content_uri = cp.content_uri AND ct.tag_id != cp.tag_id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut seen = HashSet::new();
    let edges: Vec<GraphEdge> = edge_rows
        .into_iter()
        .filter(|e| seen.insert((e.from_tag.clone(), e.to_tag.clone(), e.prereq_type.clone())))
        .filter_map(|e| {
            let from = tag_to_label.get(&e.from_tag)?.clone();
            let to = tag_to_label.get(&e.to_tag)?.clone();
            Some(GraphEdge { from, to, edge_type: e.prereq_type })
        })
        .collect();

    let nodes = tag_rows
        .into_iter()
        .map(|r| GraphNode {
            lit: lit_tags.contains(&r.tag_id),
            id: r.label_id,
            name: r.name,
            names: r.names.0,
        })
        .collect();

    Ok(GraphData { nodes, edges })
}
