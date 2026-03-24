use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};

use crate::error::ApiResult;
use crate::state::AppState;
use super::{DidQuery, RequireAuth};

#[derive(serde::Deserialize)]
pub struct FollowInput {
    did: String,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct FollowedUser {
    follows_did: String,
    handle: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
    has_update: bool,
}

pub async fn list_follows(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
) -> ApiResult<Json<Vec<FollowedUser>>> {
    let rows = sqlx::query_as::<_, FollowedUser>(
        "SELECT f.follows_did, p.handle, p.display_name, p.avatar_url, \
         CASE WHEN EXISTS ( \
           SELECT 1 FROM articles a WHERE a.did = f.follows_did \
           AND a.created_at > COALESCE( \
             (SELECT last_seen_at FROM follow_seen WHERE did = f.did AND follows_did = f.follows_did), \
             f.created_at \
           ) \
         ) THEN 1 ELSE 0 END AS has_update \
         FROM user_follows f \
         LEFT JOIN profiles p ON f.follows_did = p.did \
         WHERE f.did = ? \
         ORDER BY f.created_at DESC"
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn follow(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<FollowInput>,
) -> ApiResult<StatusCode> {
    sqlx::query(
        "INSERT OR IGNORE INTO user_follows (did, follows_did) VALUES (?, ?)"
    )
    .bind(&did)
    .bind(&input.did)
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::OK)
}

pub async fn unfollow(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<FollowInput>,
) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM user_follows WHERE did = ? AND follows_did = ?")
        .bind(&did)
        .bind(&input.did)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::OK)
}

pub async fn mark_seen(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<FollowInput>,
) -> ApiResult<StatusCode> {
    sqlx::query(
        "INSERT INTO follow_seen (did, follows_did, last_seen_at) VALUES (?, ?, datetime('now'))
         ON CONFLICT(did, follows_did) DO UPDATE SET last_seen_at = datetime('now')"
    )
    .bind(&did)
    .bind(&input.did)
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::OK)
}

/// Public: list who a user follows
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct FollowEntry {
    did: String,
    handle: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

pub async fn following_by_did(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<Vec<FollowEntry>>> {
    let rows = sqlx::query_as::<_, FollowEntry>(
        "SELECT f.follows_did AS did, p.handle, p.display_name, p.avatar_url \
         FROM user_follows f \
         LEFT JOIN profiles p ON f.follows_did = p.did \
         WHERE f.did = ? \
         ORDER BY f.created_at DESC"
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

/// Public: list who follows a user (followers)
pub async fn followers_by_did(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<Vec<FollowEntry>>> {
    let rows = sqlx::query_as::<_, FollowEntry>(
        "SELECT f.did AS did, p.handle, p.display_name, p.avatar_url \
         FROM user_follows f \
         LEFT JOIN profiles p ON f.did = p.did \
         WHERE f.follows_did = ? \
         ORDER BY f.created_at DESC"
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}
