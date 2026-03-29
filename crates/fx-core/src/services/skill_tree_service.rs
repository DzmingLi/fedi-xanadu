use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::Result;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SkillTreeRow {
    pub at_uri: String,
    pub did: String,
    pub title: String,
    pub description: Option<String>,
    pub tag_id: Option<String>,
    pub forked_from: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SkillTreeListRow {
    pub at_uri: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub tag_id: Option<String>,
    pub tag_name: Option<String>,
    #[ts(type = "Record<string, string> | null")]

    pub tag_names: Option<sqlx::types::Json<HashMap<String, String>>>,
    pub forked_from: Option<String>,
    pub created_at: DateTime<Utc>,
    pub score: i64,
    pub edge_count: i64,
    pub adopt_count: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SkillTreeEdgeRow {
    pub parent_tag: String,
    pub child_tag: String,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SkillTreeDetailResponse {
    pub tree: SkillTreeRow,
    pub edges: Vec<SkillTreeEdgeRow>,
    pub tag_names_map: HashMap<String, String>,
    pub tag_names_i18n: HashMap<String, HashMap<String, String>>,
}

pub struct CreateSkillTree {
    pub title: String,
    pub description: Option<String>,
    pub tag_id: Option<String>,
    pub edges: Vec<(String, String)>,
}

const TREE_SELECT: &str = "SELECT at_uri, did, title, description, tag_id, forked_from, created_at FROM skill_trees";

pub async fn list_skill_trees(pool: &PgPool, limit: i64) -> Result<Vec<SkillTreeListRow>> {
    let rows = sqlx::query_as::<_, SkillTreeListRow>(
        "SELECT st.at_uri, st.did, p.handle AS author_handle, st.title, st.description, \
         st.tag_id, ft.name AS tag_name, ft.names AS tag_names, \
         st.forked_from, st.created_at, \
         COALESCE((SELECT SUM(CASE WHEN v.value > 0 THEN 1 WHEN v.value < 0 THEN -1 ELSE 0 END) FROM votes v WHERE v.target_uri = st.at_uri), 0) AS score, \
         (SELECT COUNT(*) FROM skill_tree_edges e WHERE e.tree_uri = st.at_uri) AS edge_count, \
         (SELECT COUNT(*) FROM user_active_tree ua WHERE ua.tree_uri = st.at_uri) AS adopt_count \
         FROM skill_trees st LEFT JOIN profiles p ON st.did = p.did \
         LEFT JOIN tags ft ON st.tag_id = ft.id \
         ORDER BY score DESC, st.created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_skill_tree(pool: &PgPool, uri: &str) -> Result<SkillTreeRow> {
    sqlx::query_as::<_, SkillTreeRow>(&format!("{TREE_SELECT} WHERE at_uri = $1"))
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound {
            entity: "skill tree",
            id: uri.to_string(),
        })
}

pub async fn get_skill_tree_detail(pool: &PgPool, uri: &str) -> Result<SkillTreeDetailResponse> {
    let tree = get_skill_tree(pool, uri).await?;
    let edges = get_edges(pool, uri).await?;
    let (mut names_map, mut names_i18n) = collect_tag_names(pool, &edges).await?;
    // Also resolve the tree's tag name if present
    if let Some(ref tid) = tree.tag_id {
        if !names_map.contains_key(tid) {
            let ids = vec![tid.clone()];
            let fn_map = super::tag_service::get_tag_names(pool, &ids).await?;
            let fi_map = super::tag_service::get_tag_names_i18n(pool, &ids).await?;
            names_map.extend(fn_map);
            names_i18n.extend(fi_map);
        }
    }
    Ok(SkillTreeDetailResponse { tree, edges, tag_names_map: names_map, tag_names_i18n: names_i18n })
}

pub async fn create_skill_tree(
    pool: &PgPool,
    at_uri: &str,
    did: &str,
    input: &CreateSkillTree,
) -> Result<SkillTreeRow> {
    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO skill_trees (at_uri, did, title, description, tag_id) VALUES ($1, $2, $3, $4, $5)")
        .bind(at_uri)
        .bind(did)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.tag_id)
        .execute(&mut *tx)
        .await?;

    for (parent, child) in &input.edges {
        super::tag_service::ensure_tag(&mut *tx, parent, did).await?;
        super::tag_service::ensure_tag(&mut *tx, child, did).await?;
        sqlx::query("INSERT INTO skill_tree_edges (tree_uri, parent_tag, child_tag) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
            .bind(at_uri)
            .bind(parent)
            .bind(child)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    get_skill_tree(pool, at_uri).await
}

pub async fn fork_skill_tree(
    pool: &PgPool,
    source_uri: &str,
    new_uri: &str,
    did: &str,
) -> Result<SkillTreeRow> {
    let source = get_skill_tree(pool, source_uri).await?;
    let new_title = format!("Fork: {}", source.title);

    sqlx::query(
        "INSERT INTO skill_trees (at_uri, did, title, description, tag_id, forked_from) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(new_uri)
    .bind(did)
    .bind(&new_title)
    .bind(&source.description)
    .bind(&source.tag_id)
    .bind(source_uri)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO skill_tree_edges (tree_uri, parent_tag, child_tag) \
         SELECT $1, parent_tag, child_tag FROM skill_tree_edges WHERE tree_uri = $2",
    )
    .bind(new_uri)
    .bind(source_uri)
    .execute(pool)
    .await?;

    get_skill_tree(pool, new_uri).await
}

pub async fn add_edge(
    pool: &PgPool,
    tree_uri: &str,
    did: &str,
    parent_tag: &str,
    child_tag: &str,
) -> Result<()> {
    verify_owner(pool, tree_uri, did).await?;

    super::tag_service::ensure_tag(pool, parent_tag, did).await?;
    super::tag_service::ensure_tag(pool, child_tag, did).await?;

    sqlx::query("INSERT INTO skill_tree_edges (tree_uri, parent_tag, child_tag) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
        .bind(tree_uri)
        .bind(parent_tag)
        .bind(child_tag)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn remove_edge(
    pool: &PgPool,
    tree_uri: &str,
    did: &str,
    parent_tag: &str,
    child_tag: &str,
) -> Result<()> {
    verify_owner(pool, tree_uri, did).await?;

    sqlx::query("DELETE FROM skill_tree_edges WHERE tree_uri = $1 AND parent_tag = $2 AND child_tag = $3")
        .bind(tree_uri)
        .bind(parent_tag)
        .bind(child_tag)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn adopt_skill_tree(pool: &PgPool, did: &str, tree_uri: &str) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO user_active_tree (did, tree_uri) VALUES ($1, $2) ON CONFLICT(did) DO UPDATE SET tree_uri = EXCLUDED.tree_uri")
        .bind(did)
        .bind(tree_uri)
        .execute(&mut *tx)
        .await?;

    // Sync skill tree edges into user's personal tag tree
    sqlx::query("DELETE FROM user_tag_tree WHERE did = $1")
        .bind(did)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO user_tag_tree (did, parent_tag, child_tag) \
         SELECT $1, parent_tag, child_tag FROM skill_tree_edges WHERE tree_uri = $2 \
         ON CONFLICT DO NOTHING",
    )
    .bind(did)
    .bind(tree_uri)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn get_active_tree(pool: &PgPool, did: &str) -> Result<Option<SkillTreeDetailResponse>> {
    let tree_uri = sqlx::query_scalar::<_, String>(
        "SELECT tree_uri FROM user_active_tree WHERE did = $1",
    )
    .bind(did)
    .fetch_optional(pool)
    .await?;

    let Some(uri) = tree_uri else { return Ok(None) };

    let tree = sqlx::query_as::<_, SkillTreeRow>(&format!("{TREE_SELECT} WHERE at_uri = $1"))
        .bind(&uri)
        .fetch_optional(pool)
        .await?;
    let Some(tree) = tree else { return Ok(None) };

    let edges = get_edges(pool, &uri).await?;
    let (tag_names_map, tag_names_i18n) = collect_tag_names(pool, &edges).await?;

    Ok(Some(SkillTreeDetailResponse { tree, edges, tag_names_map, tag_names_i18n }))
}

// --- Helpers ---

async fn get_edges(pool: &PgPool, tree_uri: &str) -> Result<Vec<SkillTreeEdgeRow>> {
    let edges = sqlx::query_as::<_, SkillTreeEdgeRow>(
        "SELECT parent_tag, child_tag FROM skill_tree_edges WHERE tree_uri = $1",
    )
    .bind(tree_uri)
    .fetch_all(pool)
    .await?;
    Ok(edges)
}

/// Batch-fetch tag names and i18n names for all tags referenced in the given edges.
async fn collect_tag_names(
    pool: &PgPool,
    edges: &[SkillTreeEdgeRow],
) -> Result<(HashMap<String, String>, HashMap<String, HashMap<String, String>>)> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut tag_ids = Vec::new();
    for e in edges {
        if seen.insert(&e.parent_tag) {
            tag_ids.push(e.parent_tag.clone());
        }
        if seen.insert(&e.child_tag) {
            tag_ids.push(e.child_tag.clone());
        }
    }
    let names = super::tag_service::get_tag_names(pool, &tag_ids).await?;
    let names_i18n = super::tag_service::get_tag_names_i18n(pool, &tag_ids).await?;
    Ok((names, names_i18n))
}

async fn verify_owner(pool: &PgPool, tree_uri: &str, did: &str) -> Result<()> {
    let owner = sqlx::query_scalar::<_, String>(
        "SELECT did FROM skill_trees WHERE at_uri = $1",
    )
    .bind(tree_uri)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound {
        entity: "skill tree",
        id: tree_uri.to_string(),
    })?;

    if owner != did {
        return Err(crate::Error::Forbidden {
            action: "modify skill tree owned by another user",
        });
    }
    Ok(())
}
