use serde::Serialize;
use sqlx::PgPool;

use crate::models::UserSkill;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct TagTreeEntry {
    pub parent_tag: String,
    pub child_tag: String,
}

pub async fn list_user_skills(pool: &PgPool, did: &str) -> crate::Result<Vec<UserSkill>> {
    // user_skills is keyed on (did, tag_id) — one row per (user, concept).
    // The frontend resolves each tag_id to its canonical label per locale
    // via `tagStore.localize`.
    let skills = sqlx::query_as::<_, UserSkill>(
        "SELECT did, tag_id, status, lit_at \
         FROM user_skills WHERE did = $1 ORDER BY lit_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(skills)
}

pub async fn light_skill(
    pool: &PgPool,
    did: &str,
    tag_id: &str,
    status: &str,
) -> crate::Result<()> {
    let status = match status {
        "learning" => "learning",
        _ => "mastered",
    };

    sqlx::query(
        "INSERT INTO user_skills (did, tag_id, status) VALUES ($1, $2, $3) \
         ON CONFLICT(did, tag_id) DO UPDATE SET status = EXCLUDED.status, lit_at = NOW()",
    )
    .bind(did)
    .bind(tag_id)
    .bind(status)
    .execute(pool)
    .await?;

    if status == "mastered" {
        // Descend through the tag taxonomy so lighting a parent auto-lights
        // every descendant.
        let children: Vec<String> = sqlx::query_scalar(
            "WITH RECURSIVE descendants(tag) AS ( \
               SELECT child_tag FROM tag_parents WHERE parent_tag = $1 \
               UNION \
               SELECT tp.child_tag FROM tag_parents tp \
               JOIN descendants d ON tp.parent_tag = d.tag \
             ) SELECT tag FROM descendants",
        )
        .bind(tag_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for child in children {
            let _ = sqlx::query(
                "INSERT INTO user_skills (did, tag_id, status) VALUES ($1, $2, 'mastered') \
                 ON CONFLICT(did, tag_id) DO UPDATE SET status = 'mastered', lit_at = NOW()",
            )
            .bind(did)
            .bind(&child)
            .execute(pool)
            .await;
        }
    }

    Ok(())
}

pub async fn delete_skill(pool: &PgPool, did: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query(
        "DELETE FROM user_skills WHERE did = $1 AND tag_id = $2",
    )
        .bind(did)
        .bind(tag_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_user_tag_tree(pool: &PgPool, _did: &str) -> crate::Result<Vec<TagTreeEntry>> {
    // The belongs-to hierarchy is global and tag-level. For each edge,
    // surface the canonical label on either side so the tree renders
    // one node per concept rather than per-label alias.
    let tree = sqlx::query_as::<_, TagTreeEntry>(
        "SELECT tag_canonical_label(parent_tag) AS parent_tag, \
                tag_canonical_label(child_tag)  AS child_tag \
         FROM tag_parents \
         ORDER BY 1, 2",
    )
    .fetch_all(pool)
    .await?;
    Ok(tree)
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct UserTagPrereq {
    pub from_tag: String,
    pub to_tag: String,
    pub prereq_type: String,
}

pub async fn get_user_tag_prereqs(pool: &PgPool, did: &str) -> crate::Result<Vec<UserTagPrereq>> {
    // Each user owns their own prereq definitions.
    // When adopting a community tree, its prereqs are copied into this table.
    let prereqs = sqlx::query_as::<_, UserTagPrereq>(
        "SELECT from_tag, to_tag, prereq_type FROM user_tag_prereqs WHERE did = $1",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;

    Ok(prereqs)
}

pub async fn add_tag_child(
    pool: &PgPool,
    did: &str,
    parent_tag: &str,
    child_tag: &str,
) -> crate::Result<()> {
    // Hierarchy is global — delegate to tag_hierarchy_service so the edit
    // gets recorded in the audit log.
    crate::services::tag_hierarchy_service::add_edge(pool, parent_tag, child_tag, did).await
}

