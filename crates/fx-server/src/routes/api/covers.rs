//! Article / series cover images.
//!
//! Two ways to have a cover:
//! * **Upload** — bytes are written to the pijul repo as `cover.{ext}` at the
//!   repo root and a patch is recorded. Default file name, always overwrites.
//! * **Reference** — `cover_file` in the DB names an arbitrary file already in
//!   the repo (typically a body image the author already uploaded). No new
//!   bytes, no duplicate patch.
//!
//! URL scheme: `/api/covers/{kind}-{node_id}` where kind is `a` (article) or
//! `s` (series). `get_cover` reads the DB row to find `cover_file`, and falls
//! back to scanning `cover.{ext}` if the column is NULL (legacy rows + the
//! case where the repo has a `cover.png` but the DB wasn't updated).
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

const UPLOAD_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "svg"];
/// Broader allow-list for the reference path: body images in the repo may be
/// any of these. Kept in sync with `articles::get_image`'s content-type map.
const REFERENCE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "svg"];
const MAX_COVER_SIZE: usize = 5 * 1024 * 1024; // 5 MB
const COVER_STEM: &str = "cover";

fn content_type_for_ext(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        _ => "image/jpeg",
    }
}

fn ext_of(path: &str) -> String {
    std::path::Path::new(path)
        .extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase())
        .unwrap_or_default()
}

/// Reject traversal, absolute paths, and disallowed extensions.
fn validate_reference_path(file: &str) -> Result<String, AppError> {
    if file.is_empty() || file.starts_with('/') || file.contains("..") {
        return Err(AppError(fx_core::Error::BadRequest("invalid cover path".into())));
    }
    let ext = ext_of(file);
    if !REFERENCE_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest(
            "cover extension must be jpg, png, webp, gif, or svg".into(),
        )));
    }
    Ok(ext)
}

/// Resolve the key prefix + node_id to a pijul repo path.
fn repo_for(state: &AppState, kind: &str, node_id: &str) -> Option<std::path::PathBuf> {
    match kind {
        "a" => Some(state.pijul.repo_path(node_id)),
        "s" => Some(state.pijul.series_repo_path(node_id)),
        _ => None,
    }
}

/// Look up `cover_file` in the DB for a given kind + node_id.
///
/// Series: `node_id` is `series_{id}`, strip the prefix and `WHERE id = $1`.
/// Articles: `node_id` is `translate(at_uri, '/:', '__')`, recover by matching.
async fn lookup_cover_file(
    pool: &sqlx::PgPool,
    kind: &str,
    node_id: &str,
) -> Option<String> {
    match kind {
        "s" => {
            let series_id = node_id.strip_prefix("series_")?;
            sqlx::query_scalar::<_, Option<String>>(
                "SELECT cover_file FROM series WHERE id = $1",
            )
            .bind(series_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .flatten()
        }
        "a" => sqlx::query_scalar::<_, Option<String>>(
            "SELECT cover_file FROM articles \
             WHERE translate(at_uri, '/:', '__') = $1",
        )
        .bind(node_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .flatten(),
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

    // Preferred path for series: meta.yaml `cover` field — survives fork/clone.
    if kind == "s" {
        if let Some(rel) = fx_core::meta::read_series_meta(&repo).and_then(|m| m.cover) {
            if !rel.is_empty() && !rel.starts_with('/') && !rel.contains("..") {
                let ext = ext_of(&rel);
                if let Ok(data) = tokio::fs::read(repo.join(&rel)).await {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type_for_ext(&ext))
                        .header(header::CACHE_CONTROL, "public, max-age=300")
                        .body(Body::from(data))
                        .unwrap();
                }
            }
        }
    }

    // Preferred path for markdown articles: content.md's YAML frontmatter.
    if kind == "a" {
        if let Ok(src) = tokio::fs::read_to_string(repo.join("content.md")).await {
            let (fm, _) = fx_core::meta::split_frontmatter(&src);
            if let Some(rel) = fm.cover {
                if !rel.is_empty() && !rel.starts_with('/') && !rel.contains("..") {
                    let ext = ext_of(&rel);
                    if let Ok(data) = tokio::fs::read(repo.join(&rel)).await {
                        return Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, content_type_for_ext(&ext))
                            .header(header::CACHE_CONTROL, "public, max-age=300")
                            .body(Body::from(data))
                            .unwrap();
                    }
                }
            }
        }
    }

    // Next: DB cover_file (articles, or legacy series rows without meta).
    if let Some(rel) = lookup_cover_file(&state.pool, kind, node_id).await {
        if !rel.is_empty() && !rel.starts_with('/') && !rel.contains("..") {
            let ext = ext_of(&rel);
            if let Ok(data) = tokio::fs::read(repo.join(&rel)).await {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, content_type_for_ext(&ext))
                    .header(header::CACHE_CONTROL, "public, max-age=300")
                    .body(Body::from(data))
                    .unwrap();
            }
        }
    }

    // Fallback: legacy rows / uploaded covers at the default name.
    for ext in UPLOAD_EXTENSIONS {
        let path = repo.join(format!("{COVER_STEM}.{ext}"));
        if let Ok(data) = tokio::fs::read(&path).await {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type_for_ext(ext))
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
    let ext = ext_of(file_name.as_deref().unwrap_or(""));
    let ext = if ext.is_empty() { "jpg".to_string() } else { ext };
    if !UPLOAD_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest("Use jpg, png, webp, or svg".into())));
    }
    Ok((data, ext))
}

/// Remove stale cover files with other extensions so a later reader doesn't
/// pick up an outdated image when the author swaps formats.
async fn purge_other_exts(dir: &std::path::Path, keep: &str) {
    for ext in UPLOAD_EXTENSIONS {
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

    // Persist the cover selection into meta.yaml for series so fork/clone
    // preserves it without needing the database.
    if is_series {
        if let Err(e) = fx_core::meta::set_series_meta_cover(repo_path, Some(format!("{COVER_STEM}.{ext}"))) {
            tracing::warn!("update series meta cover failed: {e}");
        }
    }

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

/// For a markdown article, rewrite `content.md`'s YAML frontmatter so its
/// `cover` field matches the new selection. No-op for typst/html until those
/// formats get their own metadata mechanism. Returns whether the file changed
/// (so the caller can emit a pijul patch).
async fn update_article_cover_meta(
    pool: &sqlx::PgPool,
    repo_path: &std::path::Path,
    article_uri: &str,
    cover: Option<String>,
) -> Result<bool, AppError> {
    let format: Option<String> = sqlx::query_scalar(
        "SELECT content_format::text FROM articles WHERE at_uri = $1",
    )
    .bind(article_uri)
    .fetch_optional(pool)
    .await?;
    if format.as_deref() != Some("markdown") {
        return Ok(false);
    }
    let content_path = repo_path.join("content.md");
    let src = match tokio::fs::read_to_string(&content_path).await {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    let new_src = fx_core::meta::rewrite_markdown_cover(&src, cover);
    if new_src == src {
        return Ok(false);
    }
    tokio::fs::write(&content_path, new_src).await?;
    Ok(true)
}

/// After updating the article's markdown frontmatter cover, commit the change
/// to the repo so fork/clone carries it. Best-effort; errors are logged.
async fn record_article_cover_meta(state: &AppState, node_id: &str, did: &str) {
    if let Err(e) = state.pijul_record(node_id.to_string(), "Update cover metadata".into(), Some(did.to_string())).await {
        tracing::warn!("pijul record cover meta failed for {node_id}: {e}");
    }
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
    if is_series {
        if let Err(e) = fx_core::meta::set_series_meta_cover(repo_path, None) {
            tracing::warn!("clear series meta cover failed: {e}");
        } else {
            removed_any = true;
        }
    }
    for ext in UPLOAD_EXTENSIONS {
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

#[derive(serde::Deserialize)]
pub struct IdQuery { pub id: String }

#[derive(serde::Deserialize)]
pub struct CoverReference { pub file: String }

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
    let cover_file = format!("{COVER_STEM}.{ext}");
    if update_article_cover_meta(&state.pool, &repo_path, &q.uri, Some(cover_file.clone())).await.unwrap_or(false) {
        record_article_cover_meta(&state, &node_id, &user.did).await;
    }
    sqlx::query("UPDATE articles SET cover_url = $1, cover_file = $2 WHERE at_uri = $3")
        .bind(&cover_url).bind(&cover_file).bind(&q.uri).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url, "cover_file": cover_file })))
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
    if update_article_cover_meta(&state.pool, &repo_path, &q.uri, None).await.unwrap_or(false) {
        record_article_cover_meta(&state, &node_id, &user.did).await;
    }
    sqlx::query("UPDATE articles SET cover_url = NULL, cover_file = NULL WHERE at_uri = $1")
        .bind(&q.uri).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Point the article's cover at an existing file in its pijul repo (e.g. a
/// body image the author already uploaded). Does not touch the filesystem.
pub async fn set_article_cover_reference(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<UriQuery>,
    Json(body): Json<CoverReference>,
) -> ApiResult<Json<serde_json::Value>> {
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM articles WHERE at_uri = $1")
        .bind(&q.uri).fetch_optional(&state.pool).await?;
    let Some(owner_did) = owner else {
        return Err(AppError(fx_core::Error::NotFound { entity: "article", id: q.uri.clone() }));
    };
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the author can set a cover".into())));
    }
    set_article_cover_reference_inner(&state, &q.uri, &body.file).await
}

async fn set_article_cover_reference_inner(
    state: &AppState,
    uri: &str,
    file: &str,
) -> ApiResult<Json<serde_json::Value>> {
    let _ = validate_reference_path(file)?;
    let node_id = uri_to_node_id(uri);
    let repo_path = state.pijul.repo_path(&node_id);
    if !tokio::fs::try_exists(repo_path.join(file)).await.unwrap_or(false) {
        return Err(AppError(fx_core::Error::BadRequest(
            format!("cover file not found in repo: {file}"),
        )));
    }
    let cover_url = format!("/api/covers/a-{node_id}");
    if update_article_cover_meta(&state.pool, &repo_path, uri, Some(file.to_string())).await.unwrap_or(false) {
        let did: Option<String> = sqlx::query_scalar("SELECT did FROM articles WHERE at_uri = $1")
            .bind(uri).fetch_optional(&state.pool).await.ok().flatten();
        if let Some(d) = did {
            record_article_cover_meta(state, &node_id, &d).await;
        }
    }
    sqlx::query("UPDATE articles SET cover_url = $1, cover_file = $2 WHERE at_uri = $3")
        .bind(&cover_url).bind(file).bind(uri).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url, "cover_file": file })))
}

pub async fn admin_set_article_cover_reference(
    State(state): State<AppState>,
    _admin: AdminAuth,
    axum::extract::Query(q): axum::extract::Query<UriQuery>,
    Json(body): Json<CoverReference>,
) -> ApiResult<Json<serde_json::Value>> {
    // Still confirm the article exists.
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM articles WHERE at_uri = $1")
        .bind(&q.uri).fetch_optional(&state.pool).await?;
    if owner.is_none() {
        return Err(AppError(fx_core::Error::NotFound { entity: "article", id: q.uri.clone() }));
    }
    set_article_cover_reference_inner(&state, &q.uri, &body.file).await
}

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
    let cover_file = format!("{COVER_STEM}.{ext}");
    sqlx::query("UPDATE series SET cover_url = $1, cover_file = $2 WHERE id = $3")
        .bind(&cover_url).bind(&cover_file).bind(&q.id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url, "cover_file": cover_file })))
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
    sqlx::query("UPDATE series SET cover_url = NULL, cover_file = NULL WHERE id = $1")
        .bind(&q.id).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_series_cover_reference(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<IdQuery>,
    Json(body): Json<CoverReference>,
) -> ApiResult<Json<serde_json::Value>> {
    let owner: Option<String> = sqlx::query_scalar("SELECT created_by FROM series WHERE id = $1")
        .bind(&q.id).fetch_optional(&state.pool).await?;
    let Some(owner_did) = owner else {
        return Err(AppError(fx_core::Error::NotFound { entity: "series", id: q.id.clone() }));
    };
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the creator can set a cover".into())));
    }
    set_series_cover_reference_inner(&state, &q.id, &body.file).await
}

async fn set_series_cover_reference_inner(
    state: &AppState,
    series_id: &str,
    file: &str,
) -> ApiResult<Json<serde_json::Value>> {
    let _ = validate_reference_path(file)?;
    let node_id = format!("series_{series_id}");
    let repo_path = state.pijul.series_repo_path(&node_id);
    if !tokio::fs::try_exists(repo_path.join(file)).await.unwrap_or(false) {
        return Err(AppError(fx_core::Error::BadRequest(
            format!("cover file not found in repo: {file}"),
        )));
    }

    // meta.yaml is the source of truth; write it before the DB update and
    // record a patch so fork/clone inherits the selection.
    let owner_did: Option<String> = sqlx::query_scalar("SELECT created_by FROM series WHERE id = $1")
        .bind(series_id).fetch_optional(&state.pool).await?;
    if let Err(e) = fx_core::meta::set_series_meta_cover(&repo_path, Some(file.to_string())) {
        tracing::warn!("update series meta cover failed: {e}");
    }
    if let Some(d) = owner_did {
        if let Err(e) = state.pijul_record_series(node_id.clone(), "Set cover".into(), Some(d)).await {
            tracing::warn!("pijul record series cover ref failed: {e}");
        }
    }

    let cover_url = format!("/api/covers/s-{node_id}");
    sqlx::query("UPDATE series SET cover_url = $1, cover_file = $2 WHERE id = $3")
        .bind(&cover_url).bind(file).bind(series_id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url, "cover_file": file })))
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
    let cover_file = format!("{COVER_STEM}.{ext}");
    sqlx::query("UPDATE series SET cover_url = $1, cover_file = $2 WHERE id = $3")
        .bind(&cover_url).bind(&cover_file).bind(&q.id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url, "cover_file": cover_file })))
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
    sqlx::query("UPDATE series SET cover_url = NULL, cover_file = NULL WHERE id = $1")
        .bind(&q.id).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn admin_set_series_cover_reference(
    State(state): State<AppState>,
    _admin: AdminAuth,
    axum::extract::Query(q): axum::extract::Query<IdQuery>,
    Json(body): Json<CoverReference>,
) -> ApiResult<Json<serde_json::Value>> {
    let exists: Option<String> = sqlx::query_scalar("SELECT created_by FROM series WHERE id = $1")
        .bind(&q.id).fetch_optional(&state.pool).await?;
    if exists.is_none() {
        return Err(AppError(fx_core::Error::NotFound { entity: "series", id: q.id.clone() }));
    }
    set_series_cover_reference_inner(&state, &q.id, &body.file).await
}
