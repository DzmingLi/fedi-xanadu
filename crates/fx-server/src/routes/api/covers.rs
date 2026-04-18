//! Article / series cover images.
//!
//! Stored inside the pijul repo as `cover.{ext}` at the repo root so they
//! ride along with content: versioned, forkable, and transported via pijul
//! patches like `content.{ext}` and `description.{ext}`.
//!
//! URL scheme: `/api/covers/{kind}-{node_id}` where kind is `a` (article)
//! or `s` (series). `node_id` is `uri_to_node_id(at_uri)` for articles and
//! `series_{series_id}` for series — identical to the pijul repo folder
//! name, so resolution is a direct lookup with no DB round-trip.
use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, State},
    http::{StatusCode, Response, header},
};
use fx_core::util::uri_to_node_id;

use crate::auth::{AdminAuth, WriteAuth};
use crate::error::{AppError, ApiResult};
use crate::state::AppState;

const COVER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const MAX_COVER_SIZE: usize = 5 * 1024 * 1024; // 5 MB
const COVER_STEM: &str = "cover";

fn content_type_for(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

/// Resolve the key prefix + node_id to a pijul repo path.
fn repo_for(state: &AppState, kind: &str, node_id: &str) -> Option<std::path::PathBuf> {
    match kind {
        "a" => Some(state.pijul.repo_path(node_id)),
        "s" => Some(state.pijul.series_repo_path(node_id)),
        _ => None,
    }
}

pub async fn get_cover(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response<Body> {
    let (kind, node_id) = match id.split_once('-') {
        Some((k, n)) if !n.is_empty() => (k, n),
        _ => return Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap(),
    };
    let Some(repo) = repo_for(&state, kind, node_id) else {
        return Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap();
    };
    for ext in COVER_EXTENSIONS {
        let path = repo.join(format!("{COVER_STEM}.{ext}"));
        if let Ok(data) = tokio::fs::read(&path).await {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type_for(ext))
                .header(header::CACHE_CONTROL, "public, max-age=300")
                .body(Body::from(data))
                .unwrap();
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

/// Remove stale cover files with other extensions so a later reader doesn't
/// pick up an outdated image when the author swaps formats.
async fn purge_other_exts(dir: &std::path::Path, keep: &str) {
    for ext in COVER_EXTENSIONS {
        if *ext == keep { continue; }
        let _ = tokio::fs::remove_file(dir.join(format!("{COVER_STEM}.{ext}"))).await;
    }
}

/// Write the cover to the pijul repo, mirror to knot (if configured), and
/// record a patch so the change is versioned.
async fn write_cover_to_repo(
    state: &AppState,
    repo_path: &std::path::Path,
    node_id: &str,
    did: &str,
    data: &[u8],
    ext: &str,
    is_series: bool,
) -> Result<(), AppError> {
    tokio::fs::create_dir_all(repo_path).await.ok();
    purge_other_exts(repo_path, ext).await;
    let cover_path = repo_path.join(format!("{COVER_STEM}.{ext}"));
    tokio::fs::write(&cover_path, data).await?;

    // Mirror to knot if the user has one configured (articles only — series
    // knot flow isn't currently wired up for writes).
    if !is_series {
        let knot_url = super::articles::get_user_knot_url(&state.pool, did).await;
        if let Some(ref knot) = knot_url {
            let client = pijul_knot::KnotClient::new(knot);
            let rel = format!("{COVER_STEM}.{ext}");
            if let Err(e) = client.write_file(node_id, &rel, data).await {
                tracing::warn!("knot write cover failed: {e}");
            }
        }
    }

    // Record patch (non-fatal if it fails — the file is still on disk).
    let record = if is_series {
        state.pijul_record_series(node_id.to_string(), "Update cover".into(), Some(did.to_string())).await
    } else {
        state.pijul_record(node_id.to_string(), "Update cover".into(), Some(did.to_string())).await
    };
    if let Err(e) = record {
        tracing::warn!("pijul record cover failed for {node_id}: {e}");
    }
    Ok(())
}

/// Delete every cover.{ext} in the repo and record the deletion.
async fn remove_cover_from_repo(
    state: &AppState,
    repo_path: &std::path::Path,
    node_id: &str,
    did: &str,
    is_series: bool,
) {
    let mut removed_any = false;
    for ext in COVER_EXTENSIONS {
        if tokio::fs::remove_file(repo_path.join(format!("{COVER_STEM}.{ext}"))).await.is_ok() {
            removed_any = true;
        }
    }
    if !removed_any { return; }
    let record = if is_series {
        state.pijul_record_series(node_id.to_string(), "Remove cover".into(), Some(did.to_string())).await
    } else {
        state.pijul_record(node_id.to_string(), "Remove cover".into(), Some(did.to_string())).await
    };
    if let Err(e) = record {
        tracing::warn!("pijul record cover removal failed for {node_id}: {e}");
    }
}

#[derive(serde::Deserialize)]
pub struct UriQuery { pub uri: String }

pub async fn upload_article_cover(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<UriQuery>,
    multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM articles WHERE at_uri = $1")
        .bind(&q.uri).fetch_optional(&state.pool).await?;
    let Some(owner_did) = owner else {
        return Err(AppError(fx_core::Error::NotFound { entity: "article", id: q.uri.clone() }));
    };
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the author can set a cover".into())));
    }

    let node_id = uri_to_node_id(&q.uri);
    let repo_path = state.pijul.repo_path(&node_id);
    let (data, ext) = read_multipart(multipart).await?;

    write_cover_to_repo(&state, &repo_path, &node_id, &user.did, &data, &ext, false).await?;

    let cover_url = format!("/api/covers/a-{node_id}");
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
    let node_id = uri_to_node_id(&q.uri);
    let repo_path = state.pijul.repo_path(&node_id);
    remove_cover_from_repo(&state, &repo_path, &node_id, &user.did, false).await;
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

    let node_id = format!("series_{}", q.id);
    let repo_path = state.pijul.series_repo_path(&node_id);
    let (data, ext) = read_multipart(multipart).await?;

    write_cover_to_repo(&state, &repo_path, &node_id, &user.did, &data, &ext, true).await?;

    let cover_url = format!("/api/covers/s-{node_id}");
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
    let node_id = format!("series_{}", q.id);
    let repo_path = state.pijul.series_repo_path(&node_id);
    remove_cover_from_repo(&state, &repo_path, &node_id, &user.did, true).await;
    sqlx::query("UPDATE series SET cover_url = NULL WHERE id = $1")
        .bind(&q.id).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Admin override: set a series cover regardless of who created the series.
/// The pijul patch is still attributed to the series' `created_by` so the
/// repo history stays authored by the platform user, not "admin".
pub async fn admin_upload_series_cover(
    State(state): State<AppState>,
    _admin: AdminAuth,
    axum::extract::Query(q): axum::extract::Query<IdQuery>,
    multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let owner: Option<String> = sqlx::query_scalar("SELECT created_by FROM series WHERE id = $1")
        .bind(&q.id).fetch_optional(&state.pool).await?;
    let Some(owner_did) = owner else {
        return Err(AppError(fx_core::Error::NotFound { entity: "series", id: q.id.clone() }));
    };

    let node_id = format!("series_{}", q.id);
    let repo_path = state.pijul.series_repo_path(&node_id);
    let (data, ext) = read_multipart(multipart).await?;

    write_cover_to_repo(&state, &repo_path, &node_id, &owner_did, &data, &ext, true).await?;

    let cover_url = format!("/api/covers/s-{node_id}");
    sqlx::query("UPDATE series SET cover_url = $1 WHERE id = $2")
        .bind(&cover_url).bind(&q.id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url })))
}

pub async fn admin_remove_series_cover(
    State(state): State<AppState>,
    _admin: AdminAuth,
    axum::extract::Query(q): axum::extract::Query<IdQuery>,
) -> ApiResult<StatusCode> {
    let owner: Option<String> = sqlx::query_scalar("SELECT created_by FROM series WHERE id = $1")
        .bind(&q.id).fetch_optional(&state.pool).await?;
    let Some(owner_did) = owner else {
        return Err(AppError(fx_core::Error::NotFound { entity: "series", id: q.id.clone() }));
    };
    let node_id = format!("series_{}", q.id);
    let repo_path = state.pijul.series_repo_path(&node_id);
    remove_cover_from_repo(&state, &repo_path, &node_id, &owner_did, true).await;
    sqlx::query("UPDATE series SET cover_url = NULL WHERE id = $1")
        .bind(&q.id).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}
