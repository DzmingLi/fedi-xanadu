use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use fx_core::models::*;

use crate::error::ApiResult;
use crate::state::AppState;
use super::{AuthDid, TagIdQuery};

pub async fn list_user_skills(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
) -> ApiResult<Json<Vec<UserSkill>>> {
    let skills = sqlx::query_as::<_, UserSkill>(
        "SELECT did, tag_id, status, lit_at FROM user_skills WHERE did = ? ORDER BY lit_at DESC",
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(skills))
}

#[derive(serde::Deserialize)]
pub struct LightSkillInput {
    tag_id: String,
    status: Option<String>,
}

pub async fn light_skill(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    Json(input): Json<LightSkillInput>,
) -> ApiResult<StatusCode> {
    let status = match input.status.as_deref() {
        Some("learning") => "learning",
        _ => "mastered",
    };

    sqlx::query(
        "INSERT INTO user_skills (did, tag_id, status) VALUES (?, ?, ?)
         ON CONFLICT(did, tag_id) DO UPDATE SET status = excluded.status, lit_at = datetime('now')"
    )
        .bind(&did)
        .bind(&input.tag_id)
        .bind(status)
        .execute(&state.pool)
        .await?;

    if status == "mastered" {
        let children = get_all_children(&state.pool, &did, &input.tag_id).await;
        for child_id in children {
            let _ = sqlx::query(
                "INSERT INTO user_skills (did, tag_id, status) VALUES (?, ?, 'mastered')
                 ON CONFLICT(did, tag_id) DO UPDATE SET status = 'mastered', lit_at = datetime('now')"
            )
                .bind(&did)
                .bind(&child_id)
                .execute(&state.pool)
                .await;
        }
    }

    Ok(StatusCode::OK)
}

pub async fn delete_skill(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    Json(input): Json<TagIdQuery>,
) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM user_skills WHERE did = ? AND tag_id = ?")
        .bind(&did)
        .bind(&input.tag_id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::OK)
}

// --- User Tag Tree ---

#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct TagTreeEntry {
    parent_tag: String,
    child_tag: String,
}

pub async fn get_user_tag_tree(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
) -> ApiResult<Json<Vec<TagTreeEntry>>> {
    // First try the user's personal tag tree
    let tree = sqlx::query_as::<_, TagTreeEntry>(
        "SELECT parent_tag, child_tag FROM user_tag_tree WHERE did = ?",
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;

    if !tree.is_empty() {
        return Ok(Json(tree));
    }

    // Fall back to the user's active skill tree edges
    let tree = sqlx::query_as::<_, TagTreeEntry>(
        "SELECT e.parent_tag, e.child_tag \
         FROM skill_tree_edges e \
         JOIN user_active_tree ua ON ua.tree_uri = e.tree_uri \
         WHERE ua.did = ?",
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(tree))
}

#[derive(serde::Deserialize)]
pub struct AddTagChildInput {
    parent_tag: String,
    child_tag: String,
}

pub async fn add_tag_child(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    Json(input): Json<AddTagChildInput>,
) -> ApiResult<StatusCode> {
    sqlx::query(
        "INSERT OR IGNORE INTO user_tag_tree (did, parent_tag, child_tag) VALUES (?, ?, ?)",
    )
    .bind(&did)
    .bind(&input.parent_tag)
    .bind(&input.child_tag)
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::CREATED)
}

async fn get_all_children(
    pool: &sqlx::SqlitePool,
    did: &str,
    parent_tag: &str,
) -> Vec<String> {
    let mut result = Vec::new();
    let mut stack = vec![parent_tag.to_string()];

    while let Some(current) = stack.pop() {
        let children: Vec<String> = sqlx::query_scalar(
            "SELECT child_tag FROM user_tag_tree WHERE did = ? AND parent_tag = ?",
        )
        .bind(did)
        .bind(&current)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for child in children {
            result.push(child.clone());
            stack.push(child);
        }
    }

    result
}
