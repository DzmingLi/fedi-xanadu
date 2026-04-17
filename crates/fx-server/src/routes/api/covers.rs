//! Article / series cover images. Stored under `{data_dir}/covers/` as
//! `a-{article_rkey}.{ext}` (article) or `s-{series_id}.{ext}` (series).
//! The `/api/covers/{id}` endpoint serves them; absence = 404 = render no
//! image (PostCard has no placeholder).
use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, State},
    http::{StatusCode, Response, header},
};

use crate::auth::WriteAuth;
use crate::error::{AppError, ApiResult};
use crate::state::AppState;

const COVER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const MAX_COVER_SIZE: usize = 5 * 1024 * 1024; // 5 MB

/// Sanitize a string to `[a-zA-Z0-9_-]` for safe use as a filename.
fn safe(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect()
}

/// Article URIs are at://did/collection/rkey. Return just the rkey.
fn rkey_of(uri: &str) -> Option<&str> {
    uri.rsplit('/').next().filter(|s| !s.is_empty())
}

pub async fn get_cover(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response<Body> {
    let safe_id = safe(&id);
    let dir = state.data_dir.join("covers");
    for ext in COVER_EXTENSIONS {
        let path = dir.join(format!("{safe_id}.{ext}"));
        if path.exists() {
            let ct = match *ext {
                "png" => "image/png",
                "webp" => "image/webp",
                _ => "image/jpeg",
            };
            if let Ok(data) = tokio::fs::read(&path).await {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, ct)
                    .header(header::CACHE_CONTROL, "public, max-age=86400")
                    .body(Body::from(data))
                    .unwrap();
            }
        }
    }
    Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()
}

async fn read_multipart(mut multipart: Multipart) -> Result<(Vec<u8>, String), AppError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError(fx_core::Error::BadRequest(format!("Multipart error: {e}")))
    })? {
        if field.name() == Some("file") {
            file_name = field.file_name().map(|s| s.to_string());
            file_data = Some(
                field.bytes().await
                    .map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?
                    .to_vec(),
            );
        }
    }
    let data = file_data.ok_or(AppError(fx_core::Error::BadRequest("Missing file".into())))?;
    if data.len() > MAX_COVER_SIZE {
        return Err(AppError(fx_core::Error::BadRequest("Cover too large (max 5MB)".into())));
    }
    let ext = std::path::Path::new(file_name.as_deref().unwrap_or(""))
        .extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase())
        .unwrap_or_else(|| "jpg".into());
    if !COVER_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest("Use jpg, png, or webp".into())));
    }
    Ok((data, ext))
}

/// Remove any stale cover files with other extensions so we don't end up
/// serving an outdated image after the author swaps formats.
async fn purge_other_exts(dir: &std::path::Path, key: &str, keep: &str) {
    for ext in COVER_EXTENSIONS {
        if *ext == keep { continue; }
        let _ = tokio::fs::remove_file(dir.join(format!("{key}.{ext}"))).await;
    }
}

#[derive(serde::Deserialize)]
pub struct UriQuery { pub uri: String }

/// Upload a cover for an article. Caller must be the article's author.
pub async fn upload_article_cover(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<UriQuery>,
    multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM articles WHERE at_uri = $1")
        .bind(&q.uri)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner_did) = owner else {
        return Err(AppError(fx_core::Error::NotFound { entity: "article", id: q.uri.clone() }));
    };
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the author can set a cover".into())));
    }

    let rkey = rkey_of(&q.uri).ok_or_else(|| AppError(fx_core::Error::BadRequest("bad article uri".into())))?;
    let key = format!("a-{}", safe(rkey));

    let (data, ext) = read_multipart(multipart).await?;

    let dir = state.data_dir.join("covers");
    tokio::fs::create_dir_all(&dir).await.ok();
    purge_other_exts(&dir, &key, &ext).await;
    tokio::fs::write(dir.join(format!("{key}.{ext}")), &data).await?;

    let cover_url = format!("/api/covers/{key}");
    sqlx::query("UPDATE articles SET cover_url = $1 WHERE at_uri = $2")
        .bind(&cover_url).bind(&q.uri).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url })))
}

pub async fn remove_article_cover(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<UriQuery>,
) -> ApiResult<StatusCode> {
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM articles WHERE at_uri = $1")
        .bind(&q.uri).fetch_optional(&state.pool).await?;
    let Some(owner_did) = owner else {
        return Err(AppError(fx_core::Error::NotFound { entity: "article", id: q.uri.clone() }));
    };
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the author can remove the cover".into())));
    }
    let rkey = rkey_of(&q.uri).ok_or_else(|| AppError(fx_core::Error::BadRequest("bad article uri".into())))?;
    let key = format!("a-{}", safe(rkey));
    let dir = state.data_dir.join("covers");
    for ext in COVER_EXTENSIONS {
        let _ = tokio::fs::remove_file(dir.join(format!("{key}.{ext}"))).await;
    }
    sqlx::query("UPDATE articles SET cover_url = NULL WHERE at_uri = $1")
        .bind(&q.uri).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct IdQuery { pub id: String }

pub async fn upload_series_cover(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<IdQuery>,
    multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let owner: Option<String> = sqlx::query_scalar("SELECT created_by FROM series WHERE id = $1")
        .bind(&q.id).fetch_optional(&state.pool).await?;
    let Some(owner_did) = owner else {
        return Err(AppError(fx_core::Error::NotFound { entity: "series", id: q.id.clone() }));
    };
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the creator can set a cover".into())));
    }

    let key = format!("s-{}", safe(&q.id));
    let (data, ext) = read_multipart(multipart).await?;

    let dir = state.data_dir.join("covers");
    tokio::fs::create_dir_all(&dir).await.ok();
    purge_other_exts(&dir, &key, &ext).await;
    tokio::fs::write(dir.join(format!("{key}.{ext}")), &data).await?;

    let cover_url = format!("/api/covers/{key}");
    sqlx::query("UPDATE series SET cover_url = $1 WHERE id = $2")
        .bind(&cover_url).bind(&q.id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url })))
}

pub async fn remove_series_cover(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<IdQuery>,
) -> ApiResult<StatusCode> {
    let owner: Option<String> = sqlx::query_scalar("SELECT created_by FROM series WHERE id = $1")
        .bind(&q.id).fetch_optional(&state.pool).await?;
    let Some(owner_did) = owner else {
        return Err(AppError(fx_core::Error::NotFound { entity: "series", id: q.id.clone() }));
    };
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the creator can remove the cover".into())));
    }
    let key = format!("s-{}", safe(&q.id));
    let dir = state.data_dir.join("covers");
    for ext in COVER_EXTENSIONS {
        let _ = tokio::fs::remove_file(dir.join(format!("{key}.{ext}"))).await;
    }
    sqlx::query("UPDATE series SET cover_url = NULL WHERE id = $1")
        .bind(&q.id).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}
