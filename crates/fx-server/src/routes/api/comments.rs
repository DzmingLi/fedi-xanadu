use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::models::Comment;

use crate::error::{ApiError, ApiResult, require_owner};
use crate::state::AppState;
use super::{AuthDid, RequireAuth, UriQuery, tid};

const COMMENT_SELECT: &str = "\
    SELECT c.id, c.article_uri, c.did, p.handle AS author_handle, c.parent_id, c.body, \
    COALESCE((SELECT SUM(value) FROM comment_votes WHERE comment_id = c.id), 0) AS vote_score, \
    c.created_at, c.updated_at \
    FROM comments c LEFT JOIN profiles p ON c.did = p.did";

pub async fn list_comments(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<Comment>>> {
    let comments = sqlx::query_as::<_, Comment>(
        &format!("{COMMENT_SELECT} WHERE c.article_uri = ? ORDER BY c.created_at ASC"),
    )
    .bind(&uri)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(comments))
}

#[derive(serde::Deserialize)]
pub struct CreateComment {
    pub article_uri: String,
    pub body: String,
    pub parent_id: Option<String>,
}

pub async fn create_comment(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<CreateComment>,
) -> ApiResult<(StatusCode, Json<Comment>)> {
    // If replying, verify parent exists and belongs to the same article
    if let Some(ref pid) = input.parent_id {
        let parent_uri: Option<String> =
            sqlx::query_scalar("SELECT article_uri FROM comments WHERE id = ?")
                .bind(pid)
                .fetch_optional(&state.pool)
                .await?;
        match parent_uri {
            Some(uri) if uri == input.article_uri => {}
            Some(_) => return Err(ApiError::BadRequest("parent comment belongs to a different article".into())),
            None => return Err(ApiError::NotFound("parent comment not found".into())),
        }
    }

    let id = tid();

    sqlx::query(
        "INSERT INTO comments (id, article_uri, did, body, parent_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.article_uri)
    .bind(&did)
    .bind(&input.body)
    .bind(&input.parent_id)
    .execute(&state.pool)
    .await?;

    let comment = sqlx::query_as::<_, Comment>(
        &format!("{COMMENT_SELECT} WHERE c.id = ?"),
    )
    .bind(&id)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(comment)))
}

#[derive(serde::Deserialize)]
pub struct UpdateComment {
    pub id: String,
    pub body: String,
}

pub async fn update_comment(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<UpdateComment>,
) -> ApiResult<Json<Comment>> {
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM comments WHERE id = ?")
        .bind(&input.id)
        .fetch_optional(&state.pool)
        .await?;
    require_owner(owner.as_deref(), &did)?;

    sqlx::query("UPDATE comments SET body = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&input.body)
        .bind(&input.id)
        .execute(&state.pool)
        .await?;

    let comment = sqlx::query_as::<_, Comment>(
        &format!("{COMMENT_SELECT} WHERE c.id = ?"),
    )
    .bind(&input.id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(comment))
}

#[derive(serde::Deserialize)]
pub struct DeleteComment {
    pub id: String,
}

pub async fn delete_comment(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<DeleteComment>,
) -> ApiResult<StatusCode> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT c.did, a.did FROM comments c JOIN articles a ON a.at_uri = c.article_uri WHERE c.id = ?",
    )
    .bind(&input.id)
    .fetch_optional(&state.pool)
    .await?;

    match row {
        Some((comment_did, article_did)) if comment_did == did || article_did == did => {}
        Some(_) => return Err(ApiError::Forbidden),
        None => return Err(ApiError::NotFound("comment not found".into())),
    }

    // Delete child comments first
    sqlx::query("DELETE FROM comment_votes WHERE comment_id IN (SELECT id FROM comments WHERE parent_id = ?)")
        .bind(&input.id)
        .execute(&state.pool)
        .await?;
    sqlx::query("DELETE FROM comments WHERE parent_id = ?")
        .bind(&input.id)
        .execute(&state.pool)
        .await?;

    sqlx::query("DELETE FROM comment_votes WHERE comment_id = ?")
        .bind(&input.id)
        .execute(&state.pool)
        .await?;
    sqlx::query("DELETE FROM comments WHERE id = ?")
        .bind(&input.id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Comment votes ---

#[derive(serde::Deserialize)]
pub struct CommentVoteInput {
    pub comment_id: String,
    pub value: i32,
}

#[derive(serde::Serialize)]
pub struct CommentVoteResult {
    pub comment_id: String,
    pub score: i64,
    pub my_vote: i32,
}

pub async fn vote_comment(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<CommentVoteInput>,
) -> ApiResult<Json<CommentVoteResult>> {
    // Clamp to -1..1
    let value = input.value.clamp(-1, 1);

    if value == 0 {
        sqlx::query("DELETE FROM comment_votes WHERE comment_id = ? AND did = ?")
            .bind(&input.comment_id)
            .bind(&did)
            .execute(&state.pool)
            .await?;
    } else {
        sqlx::query(
            "INSERT INTO comment_votes (comment_id, did, value) VALUES (?, ?, ?)
             ON CONFLICT(comment_id, did) DO UPDATE SET value = excluded.value",
        )
        .bind(&input.comment_id)
        .bind(&did)
        .bind(value)
        .execute(&state.pool)
        .await?;
    }

    let score: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(value), 0) FROM comment_votes WHERE comment_id = ?",
    )
    .bind(&input.comment_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(CommentVoteResult {
        comment_id: input.comment_id,
        score,
        my_vote: value,
    }))
}

#[derive(serde::Deserialize)]
pub struct CommentIdQuery {
    pub comment_id: String,
}

pub async fn get_my_comment_votes(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<MyCommentVote>>> {
    let votes = sqlx::query_as::<_, MyCommentVote>(
        "SELECT cv.comment_id, cv.value FROM comment_votes cv \
         JOIN comments c ON c.id = cv.comment_id \
         WHERE cv.did = ? AND c.article_uri = ?",
    )
    .bind(&did)
    .bind(&uri)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(votes))
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct MyCommentVote {
    pub comment_id: String,
    pub value: i32,
}
