use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use fx_core::services::bookmark_service;
use fx_core::validation;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::{Auth, WriteAuth, pds_put_record, pds_delete_record};
use fx_core::util::now_rfc3339;
use super::DidQuery;

fn article_rkey(uri: &str) -> Option<String> {
    uri.rsplit('/').next().map(str::to_string)
}

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
    WriteAuth(user): WriteAuth,
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

    if let Some(rkey) = article_rkey(&input.article_uri) {
        let (subject, section_ref) =
            fx_core::services::article_service::resolve_subject_ref(&state.pool, &input.article_uri, fx_atproto::lexicon::WORK).await;
        let mut record = serde_json::json!({
            "$type": fx_atproto::lexicon::BOOKMARK,
            "subject": subject,
            "folderPath": folder,
            "createdAt": now_rfc3339(),
        });
        if let Some(sr) = section_ref { record["sectionRef"] = serde_json::Value::String(sr); }
        pds_put_record(&state, &user.token, fx_atproto::lexicon::BOOKMARK, rkey, record, "bookmark add").await;
    }

    Ok(StatusCode::CREATED)
}

#[derive(serde::Deserialize)]
pub struct RemoveBookmarkInput {
    uri: String,
}

pub async fn remove_bookmark(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<RemoveBookmarkInput>,
) -> ApiResult<StatusCode> {
    bookmark_service::remove_bookmark(&state.pool, &user.did, &input.uri).await?;

    if let Some(rkey) = article_rkey(&input.uri) {
        pds_delete_record(&state, &user.token, fx_atproto::lexicon::BOOKMARK, rkey, "bookmark remove").await;
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub(crate) struct MoveBookmarkInput {
    article_uri: String,
    folder_path: String,
}

pub async fn move_bookmark(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<MoveBookmarkInput>,
) -> ApiResult<StatusCode> {
    if let Err(e) = validation::validate_folder_path(&input.folder_path) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    bookmark_service::move_bookmark(&state.pool, &user.did, &input.article_uri, &input.folder_path)
        .await?;

    if let Some(rkey) = article_rkey(&input.article_uri) {
        let (subject, section_ref) =
            fx_core::services::article_service::resolve_subject_ref(&state.pool, &input.article_uri, fx_atproto::lexicon::WORK).await;
        let mut record = serde_json::json!({
            "$type": fx_atproto::lexicon::BOOKMARK,
            "subject": subject,
            "folderPath": input.folder_path,
            "createdAt": now_rfc3339(),
        });
        if let Some(sr) = section_ref { record["sectionRef"] = serde_json::Value::String(sr); }
        pds_put_record(&state, &user.token, fx_atproto::lexicon::BOOKMARK, rkey, record, "bookmark move").await;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_bookmark_folders(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<String>>> {
    let folders = bookmark_service::list_bookmark_folders(&state.pool, &user.did).await?;
    Ok(Json(folders))
}

/// View another user's public bookmarks (respects their visibility settings).
pub async fn list_public_bookmarks(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<DidQuery>,
) -> ApiResult<Json<Vec<bookmark_service::BookmarkWithTitle>>> {
    // Check if the user has bookmarks_public enabled
    #[derive(sqlx::FromRow)]
    struct VisRow {
        bookmarks_public: bool,
        public_folders: sqlx::types::JsonValue,
    }

    let vis = sqlx::query_as::<_, VisRow>(
        "SELECT COALESCE(bookmarks_public, false) AS bookmarks_public, \
         COALESCE(public_folders, '[]') AS public_folders \
         FROM user_settings WHERE did = $1",
    )
    .bind(&q.did)
    .fetch_optional(&state.pool)
    .await?;

    let (is_public, folders) = match vis {
        Some(v) => {
            let f: Vec<String> = serde_json::from_value(v.public_folders).unwrap_or_default();
            (v.bookmarks_public, f)
        }
        None => (false, Vec::new()),
    };

    if !is_public {
        return Ok(Json(Vec::new()));
    }

    let all = bookmark_service::list_bookmarks(&state.pool, &q.did).await?;

    // If specific folders are set, only return those; otherwise return all
    if folders.is_empty() {
        Ok(Json(all))
    } else {
        let filtered = all
            .into_iter()
            .filter(|b| folders.iter().any(|f| b.folder_path.starts_with(f)))
            .collect();
        Ok(Json(filtered))
    }
}
