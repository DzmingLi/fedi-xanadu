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

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SkillTreePrereqRow {
    pub from_tag: String,
    pub to_tag: String,
    pub prereq_type: String,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SkillTreeDetailResponse {
    pub tree: SkillTreeRow,
    pub edges: Vec<SkillTreeEdgeRow>,
    pub prereqs: Vec<SkillTreePrereqRow>,
    pub tag_names_map: HashMap<String, String>,
    pub tag_names_i18n: HashMap<String, HashMap<String, String>>,
}

pub struct CreateSkillTree {
    pub title: String,
    pub description: Option<String>,
    pub tag_id: Option<String>,
    pub edges: Vec<(String, String)>,
    pub prereqs: Vec<(String, String, String)>,
}

const TREE_SELECT: &str = "SELECT at_uri, did, title, description, tag_id, forked_from, created_at FROM skill_trees";

pub async fn list_skill_trees(pool: &PgPool, limit: i64) -> Result<Vec<SkillTreeListRow>> {
    // `skill_trees.tag_id` is a tag (concept); surface its canonical label
    // id as the row's `tag_id` so the frontend (which routes via label
    // ids) keeps working, and attach the full translation map.
    let rows = sqlx::query_as::<_, SkillTreeListRow>(
        "SELECT st.at_uri, st.did, p.handle AS author_handle, st.title, st.description, \
         tag_canonical_label(st.tag_id) AS tag_id, \
         tag_canonical_label(st.tag_id) AS tag_name, \
         tag_label_map(st.tag_id) AS tag_names, \
         st.forked_from, st.created_at, \
         COALESCE((SELECT SUM(CASE WHEN v.value > 0 THEN 1 WHEN v.value < 0 THEN -1 ELSE 0 END) FROM votes v WHERE v.target_uri = st.at_uri), 0) AS score, \
         (SELECT COUNT(*) FROM skill_tree_edges e WHERE e.tree_uri = st.at_uri) AS edge_count, \
         (SELECT COUNT(*) FROM user_active_tree ua WHERE ua.tree_uri = st.at_uri) AS adopt_count \
         FROM skill_trees st LEFT JOIN profiles p ON st.did = p.did \
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
    let prereqs = get_prereqs(pool, uri).await?;
    let (mut names_map, mut names_i18n) = collect_tag_names(pool, &edges, &prereqs).await?;
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
    Ok(SkillTreeDetailResponse { tree, edges, prereqs, tag_names_map: names_map, tag_names_i18n: names_i18n })
}

pub async fn create_skill_tree(
    pool: &PgPool,
    at_uri: &str,
    did: &str,
    input: &CreateSkillTree,
) -> Result<SkillTreeRow> {
    let mut tx = pool.begin().await?;

    // Root tag, edges, prereqs all reference canonical tag_ids.
    // resolve_tag_id accepts tag_id, label id, or new name.
    let root_tag_id = if let Some(ref input_ref) = input.tag_id {
        Some(super::tag_service::resolve_tag_id(&mut *tx, input_ref, did).await?)
    } else {
        None
    };
    sqlx::query(
        "INSERT INTO skill_trees (at_uri, did, title, description, tag_id) \
         VALUES ($1, $2, $3, $4, $5)",
    )
        .bind(at_uri)
        .bind(did)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&root_tag_id)
        .execute(&mut *tx)
        .await?;

    for (parent, child) in &input.edges {
        let parent_id = super::tag_service::resolve_tag_id(&mut *tx, parent, did).await?;
        let child_id = super::tag_service::resolve_tag_id(&mut *tx, child, did).await?;
        sqlx::query(
            "INSERT INTO skill_tree_edges (tree_uri, parent_tag, child_tag) \
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
            .bind(at_uri)
            .bind(&parent_id)
            .bind(&child_id)
            .execute(&mut *tx)
            .await?;
    }

    for (from_tag, to_tag, prereq_type) in &input.prereqs {
        let from_id = super::tag_service::resolve_tag_id(&mut *tx, from_tag, did).await?;
        let to_id = super::tag_service::resolve_tag_id(&mut *tx, to_tag, did).await?;
        sqlx::query(
            "INSERT INTO skill_tree_prereqs (tree_uri, from_tag, to_tag, prereq_type) \
             VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        )
            .bind(at_uri)
            .bind(&from_id)
            .bind(&to_id)
            .bind(prereq_type)
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

    sqlx::query(
        "INSERT INTO skill_tree_prereqs (tree_uri, from_tag, to_tag, prereq_type) \
         SELECT $1, from_tag, to_tag, prereq_type FROM skill_tree_prereqs WHERE tree_uri = $2",
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
    parent_label: &str,
    child_label: &str,
) -> Result<()> {
    verify_owner(pool, tree_uri, did).await?;

    let mut conn = pool.acquire().await?;
    let parent_id = super::tag_service::resolve_tag_id(&mut conn, parent_label, did).await?;
    let child_id = super::tag_service::resolve_tag_id(&mut conn, child_label, did).await?;

    sqlx::query(
        "INSERT INTO skill_tree_edges (tree_uri, parent_tag, child_tag) \
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
        .bind(tree_uri)
        .bind(&parent_id)
        .bind(&child_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn remove_edge(
    pool: &PgPool,
    tree_uri: &str,
    did: &str,
    parent_label: &str,
    child_label: &str,
) -> Result<()> {
    verify_owner(pool, tree_uri, did).await?;

    // remove_edge accepts label id or tag_id; resolve to tag_id for the delete.
    let mut conn = pool.acquire().await?;
    let parent_id = super::tag_service::resolve_tag_id(&mut conn, parent_label, did).await?;
    let child_id = super::tag_service::resolve_tag_id(&mut conn, child_label, did).await?;

    sqlx::query(
        "DELETE FROM skill_tree_edges \
         WHERE tree_uri = $1 AND parent_tag = $2 AND child_tag = $3",
    )
        .bind(tree_uri)
        .bind(&parent_id)
        .bind(&child_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn add_prereq(
    pool: &PgPool,
    tree_uri: &str,
    did: &str,
    from_label: &str,
    to_label: &str,
    prereq_type: &str,
) -> Result<()> {
    verify_owner(pool, tree_uri, did).await?;

    let mut conn = pool.acquire().await?;
    let from_id = super::tag_service::resolve_tag_id(&mut conn, from_label, did).await?;
    let to_id = super::tag_service::resolve_tag_id(&mut conn, to_label, did).await?;

    sqlx::query(
        "INSERT INTO skill_tree_prereqs (tree_uri, from_tag, to_tag, prereq_type) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
        .bind(tree_uri)
        .bind(&from_id)
        .bind(&to_id)
        .bind(prereq_type)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn remove_prereq(
    pool: &PgPool,
    tree_uri: &str,
    did: &str,
    from_label: &str,
    to_label: &str,
) -> Result<()> {
    verify_owner(pool, tree_uri, did).await?;

    let mut conn = pool.acquire().await?;
    let from_id = super::tag_service::resolve_tag_id(&mut conn, from_label, did).await?;
    let to_id = super::tag_service::resolve_tag_id(&mut conn, to_label, did).await?;

    sqlx::query(
        "DELETE FROM skill_tree_prereqs \
         WHERE tree_uri = $1 AND from_tag = $2 AND to_tag = $3",
    )
        .bind(tree_uri)
        .bind(&from_id)
        .bind(&to_id)
        .execute(&mut *conn)
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

    // Hierarchy edges live in the global tag_parents table now — adopting
    // a skill tree only copies its subjective prereq definitions below.

    // Sync prereq edges
    sqlx::query("DELETE FROM user_tag_prereqs WHERE did = $1")
        .bind(did)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO user_tag_prereqs (did, from_tag, to_tag, prereq_type) \
         SELECT $1, from_tag, to_tag, prereq_type FROM skill_tree_prereqs WHERE tree_uri = $2 \
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
    let prereqs = get_prereqs(pool, &uri).await?;
    let (tag_names_map, tag_names_i18n) = collect_tag_names(pool, &edges, &prereqs).await?;

    Ok(Some(SkillTreeDetailResponse { tree, edges, prereqs, tag_names_map, tag_names_i18n }))
}

// --- Helpers ---

async fn get_edges(pool: &PgPool, tree_uri: &str) -> Result<Vec<SkillTreeEdgeRow>> {
    // Surface canonical labels so the frontend's label-keyed tagStore
    // can resolve them without another round-trip.
    let edges = sqlx::query_as::<_, SkillTreeEdgeRow>(
        "SELECT tag_canonical_label(parent_tag) AS parent_tag, \
                tag_canonical_label(child_tag)  AS child_tag \
         FROM skill_tree_edges WHERE tree_uri = $1",
    )
    .bind(tree_uri)
    .fetch_all(pool)
    .await?;
    Ok(edges)
}

async fn get_prereqs(pool: &PgPool, tree_uri: &str) -> Result<Vec<SkillTreePrereqRow>> {
    let prereqs = sqlx::query_as::<_, SkillTreePrereqRow>(
        "SELECT tag_canonical_label(from_tag) AS from_tag, \
                tag_canonical_label(to_tag)   AS to_tag, \
                prereq_type \
         FROM skill_tree_prereqs WHERE tree_uri = $1",
    )
    .bind(tree_uri)
    .fetch_all(pool)
    .await?;
    Ok(prereqs)
}

/// Batch-fetch tag names and i18n names for all tags referenced in edges and prereqs.
async fn collect_tag_names(
    pool: &PgPool,
    edges: &[SkillTreeEdgeRow],
    prereqs: &[SkillTreePrereqRow],
) -> Result<(HashMap<String, String>, HashMap<String, HashMap<String, String>>)> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut tag_ids = Vec::new();
    for e in edges {
        if seen.insert(e.parent_tag.as_str()) {
            tag_ids.push(e.parent_tag.clone());
        }
        if seen.insert(e.child_tag.as_str()) {
            tag_ids.push(e.child_tag.clone());
        }
    }
    for p in prereqs {
        if seen.insert(p.from_tag.as_str()) {
            tag_ids.push(p.from_tag.clone());
        }
        if seen.insert(p.to_tag.as_str()) {
            tag_ids.push(p.to_tag.clone());
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
