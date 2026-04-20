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
    // Return one row per (user, group member) — if the user lit "calculus"
    // we emit rows for "calculus" and for every sibling in the same
    // alias/translation group (e.g. "高等数学"). A `skillMap` built on
    // tag_id in the frontend then correctly marks every language label of
    // a mastered concept as mastered, without callers needing to know
    // about groups.
    let skills = sqlx::query_as::<_, UserSkill>(
        "SELECT us.did, sib.id AS tag_id, us.status, us.lit_at, sib.group_id \
         FROM user_skills us \
         JOIN tags anchor ON anchor.id = us.tag_id \
         JOIN tags sib   ON sib.group_id = anchor.group_id \
         WHERE us.did = $1 \
         ORDER BY us.lit_at DESC",
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
        let children = get_all_children(pool, did, tag_id).await;
        for child_id in children {
            let _ = sqlx::query(
                "INSERT INTO user_skills (did, tag_id, status) VALUES ($1, $2, 'mastered') \
                 ON CONFLICT(did, tag_id) DO UPDATE SET status = 'mastered', lit_at = NOW()",
            )
            .bind(did)
            .bind(&child_id)
            .execute(pool)
            .await;
        }
    }

    Ok(())
}

pub async fn delete_skill(pool: &PgPool, did: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM user_skills WHERE did = $1 AND tag_id = $2")
        .bind(did)
        .bind(tag_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_user_tag_tree(pool: &PgPool, _did: &str) -> crate::Result<Vec<TagTreeEntry>> {
    // Belongs-to hierarchy is now a single global source of truth. The
    // `did` parameter is retained for API-signature stability.
    let tree = sqlx::query_as::<_, TagTreeEntry>(
        "SELECT parent_tag, child_tag FROM tag_parents ORDER BY parent_tag, child_tag",
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

async fn get_all_children(pool: &PgPool, _did: &str, parent_tag: &str) -> Vec<String> {
    sqlx::query_scalar(
        "WITH RECURSIVE descendants(tag) AS ( \
           SELECT child_tag FROM tag_parents WHERE parent_tag = $1 \
           UNION \
           SELECT tp.child_tag FROM tag_parents tp \
           JOIN descendants d ON tp.parent_tag = d.tag \
         ) \
         SELECT tag FROM descendants",
    )
    .bind(parent_tag)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}
