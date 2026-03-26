use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use fx_core::services::bookmark_service;
use fx_core::validation;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use super::Auth;

#[derive(serde::Deserialize)]
pub struct AddBookmarkInput {
    article_uri: String,
    folder_path: Option<String>,
}

pub async fn list_bookmarks(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<bookmark_service::BookmarkWithTitle>>> {
    let rows = bookmark_service::list_bookmarks(&state.pool, &user.did).await?;
    Ok(Json(rows))
}

pub async fn add_bookmark(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<AddBookmarkInput>,
) -> ApiResult<StatusCode> {
    let folder = input.folder_path.unwrap_or_else(|| "/".to_string());

    if let Err(e) = validation::validate_at_uri(&input.article_uri) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }
    if let Err(e) = validation::validate_folder_path(&folder) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    bookmark_service::add_bookmark(&state.pool, &user.did, &input.article_uri, &folder).await?;
    Ok(StatusCode::CREATED)
}

#[derive(serde::Deserialize)]
pub struct RemoveBookmarkInput {
    uri: String,
}

pub async fn remove_bookmark(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<RemoveBookmarkInput>,
) -> ApiResult<StatusCode> {
    bookmark_service::remove_bookmark(&state.pool, &user.did, &input.uri).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub(crate) struct MoveBookmarkInput {
    article_uri: String,
    folder_path: String,
}

pub async fn move_bookmark(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<MoveBookmarkInput>,
) -> ApiResult<StatusCode> {
    if let Err(e) = validation::validate_folder_path(&input.folder_path) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    bookmark_service::move_bookmark(&state.pool, &user.did, &input.article_uri, &input.folder_path)
        .await?;
    Ok(StatusCode::OK)
}

pub async fn list_bookmark_folders(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<String>>> {
    let folders = bookmark_service::list_bookmark_folders(&state.pool, &user.did).await?;
    Ok(Json(folders))
}
