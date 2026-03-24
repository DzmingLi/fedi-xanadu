use axum::{
    Json,
    extract::State,
    http::StatusCode,
};

use crate::error::ApiResult;
use crate::state::AppState;
use super::AuthDid;

#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct BookmarkWithTitle {
    article_uri: String,
    folder_path: String,
    created_at: String,
    title: String,
    description: String,
}

pub async fn list_bookmarks(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
) -> ApiResult<Json<Vec<BookmarkWithTitle>>> {
    let rows = sqlx::query_as::<_, BookmarkWithTitle>(
        "SELECT b.article_uri, b.folder_path, b.created_at, a.title, a.description
         FROM user_bookmarks b
         JOIN articles a ON a.at_uri = b.article_uri
         WHERE b.did = ?
         ORDER BY b.folder_path, b.created_at",
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

#[derive(serde::Deserialize)]
pub struct AddBookmarkInput {
    article_uri: String,
    folder_path: Option<String>,
}

pub async fn add_bookmark(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    Json(input): Json<AddBookmarkInput>,
) -> ApiResult<StatusCode> {
    let folder = input.folder_path.unwrap_or_else(|| "/".to_string());
    sqlx::query(
        "INSERT OR REPLACE INTO user_bookmarks (did, article_uri, folder_path) VALUES (?, ?, ?)",
    )
    .bind(&did)
    .bind(&input.article_uri)
    .bind(&folder)
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::CREATED)
}

#[derive(serde::Deserialize)]
pub struct RemoveBookmarkInput {
    uri: String,
}

pub async fn remove_bookmark(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    Json(input): Json<RemoveBookmarkInput>,
) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM user_bookmarks WHERE did = ? AND article_uri = ?")
        .bind(&did)
        .bind(&input.uri)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub(crate) struct MoveBookmarkInput {
    article_uri: String,
    folder_path: String,
}

pub async fn move_bookmark(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    Json(input): Json<MoveBookmarkInput>,
) -> ApiResult<StatusCode> {
    sqlx::query("UPDATE user_bookmarks SET folder_path = ? WHERE did = ? AND article_uri = ?")
        .bind(&input.folder_path)
        .bind(&did)
        .bind(&input.article_uri)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::OK)
}

pub async fn list_bookmark_folders(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
) -> ApiResult<Json<Vec<String>>> {
    let folders: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT folder_path FROM user_bookmarks WHERE did = ? ORDER BY folder_path",
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(folders))
}
