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
use std::sync::LazyLock;

use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, State},
    http::{StatusCode, Response, header},
    response::IntoResponse,
};
use fx_core::util::uri_to_node_id;
use regex_lite::Regex;

use crate::auth::{AdminAuth, WriteAuth};
use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use fx_core::services::{article_service, series_service};

const UPLOAD_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "svg"];
/// Broader allow-list for the reference path: body images in the repo may be
/// any of these. Kept in sync with `articles::get_image`'s content-type map.
const REFERENCE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "svg"];
const MAX_COVER_SIZE: usize = 5 * 1024 * 1024; // 5 MB
const COVER_STEM: &str = "cover";
const COVER_CACHE: &str = "public, max-age=300";

static TYPST_COVER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^\s*#metadata\([^)]*\)\s*<nbt-article>\s*\n?"#)
        .expect("cover typst regex")
});
static HTML_COVER_META_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<meta\s+name="nightboat:cover"\s+content="([^"]+)"\s*/?>"#)
        .expect("cover html meta regex")
});
static HTML_COVER_META_STRIP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<meta\s+name="nightboat:cover"\s+content="[^"]*"\s*/?>\s*\n?"#)
        .expect("cover html meta strip regex")
});
static HTML_HEAD_OPEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)<head[^>]*>").expect("cover html head open regex")
});

fn empty_response(status: StatusCode) -> Response<Body> {
    status.into_response()
}

fn image_response(data: Vec<u8>, ext: &str) -> Response<Body> {
    (
        [
            (header::CONTENT_TYPE, content_type_for_ext(ext)),
            (header::CACHE_CONTROL, COVER_CACHE),
        ],
        data,
    )
        .into_response()
}

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

/// Resolve the key prefix + node_id to the blob-cache directory that holds
/// the cover file. With the move to PDS-blob storage, every article cover
/// lives in `blob_cache_path/{node_id}/`. Series still use the node_id
/// scheme but now have no on-disk content — covers must be published as
/// part of a chapter's blob bundle to be servable; for the moment series
/// covers simply use the same convention (cover file copied into a blob
/// cache dir keyed by the series' repo_uri node_id).
async fn repo_for(state: &AppState, kind: &str, node_id: &str) -> Option<std::path::PathBuf> {
    match kind {
        "a" | "s" => Some(state.blob_cache_path.join(node_id)),
        _ => None,
    }
}

/// Look up `cover_file` in the DB for a given kind + node_id. Both kinds use
/// the same translate-by-URI pattern since series node_ids are now
/// `uri_to_node_id(series_repo_uri)` — uniform with articles.
async fn lookup_cover_file(
    pool: &sqlx::PgPool,
    kind: &str,
    node_id: &str,
) -> Option<String> {
    match kind {
        "s" => sqlx::query_scalar::<_, Option<String>>(
            "SELECT cover_file FROM series \
             WHERE translate('at://' || created_by || '/at.nightbo.work/' || id, '/:', '__') = $1",
        )
        .bind(node_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .flatten(),
        "a" => sqlx::query_scalar::<_, Option<String>>(
            "SELECT a.cover_file FROM articles a \
             JOIN article_localizations l \
               ON a.repo_uri = l.repo_uri AND a.source_path = l.source_path \
             WHERE translate(l.at_uri, '/:', '__') = $1 LIMIT 1",
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
        _ => return empty_response(StatusCode::NOT_FOUND),
    };
    let Some(repo) = repo_for(&state, kind, node_id).await else {
        return empty_response(StatusCode::NOT_FOUND);
    };

    // Preferred path for series: the repo's native metadata slot.
    // Typst series (main.typ present) query the `<nbt-series>` directive;
    // markdown/html series fall back to meta.yaml.
    if kind == "s" {
        let rel = if repo.join("main.typ").exists() {
            let repo_owned = repo.clone();
            tokio::task::spawn_blocking(move || {
                let config = fx_renderer::fx_render_config();
                fx_renderer::extract_typst_series_summary(&repo_owned, &config)
            })
            .await
            .ok()
            .flatten()
            .and_then(|m| m.cover)
        } else {
            fx_core::meta::read_series_meta(&repo).and_then(|m| m.cover)
        };
        if let Some(rel) = rel {
            if !rel.is_empty() && !rel.starts_with('/') && !rel.contains("..") {
                let ext = ext_of(&rel);
                if let Ok(data) = tokio::fs::read(repo.join(&rel)).await {
                    return image_response(data, &ext);
                }
            }
        }
    }

    // Preferred path for articles: each format's native metadata.
    // Markdown → content.md frontmatter. HTML → <meta name="nightboat:cover">.
    // Typst is handled through the DB: publish_article_content extracts the
    // cover via typst introspection and syncs it to articles.cover_file.
    if kind == "a" {
        let mut rel_from_native: Option<String> = None;
        if let Ok(src) = tokio::fs::read_to_string(repo.join("content.md")).await {
            rel_from_native = fx_core::meta::split_frontmatter(&src).0.cover;
        }
        if rel_from_native.is_none() {
            if let Ok(src) = tokio::fs::read_to_string(repo.join("content.html")).await {
                rel_from_native = html_cover_from_meta(&src);
            }
        }
        if let Some(rel) = rel_from_native {
            if !rel.is_empty() && !rel.starts_with('/') && !rel.contains("..") {
                let ext = ext_of(&rel);
                if let Ok(data) = tokio::fs::read(repo.join(&rel)).await {
                    return image_response(data, &ext);
                }
            }
        }
    }

    // Next: DB cover_file (articles, or legacy series rows without meta).
    if let Some(rel) = lookup_cover_file(&state.pool, kind, node_id).await {
        if !rel.is_empty() && !rel.starts_with('/') && !rel.contains("..") {
            let ext = ext_of(&rel);
            if let Ok(data) = tokio::fs::read(repo.join(&rel)).await {
                return image_response(data, &ext);
            }
        }
    }

    // Fallback: legacy rows / uploaded covers at the default name.
    for ext in UPLOAD_EXTENSIONS {
        let path = repo.join(format!("{COVER_STEM}.{ext}"));
        if let Ok(data) = tokio::fs::read(&path).await {
            return image_response(data, ext);
        }
    }
    empty_response(StatusCode::NOT_FOUND)
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

/// Persist a series cover selection to whichever metadata slot the series
/// uses: `main.typ` with a `<nbt-series>` directive for typst series, or
/// meta.yaml otherwise.
async fn persist_series_cover(repo_path: &std::path::Path, cover: Option<String>) {
    let main_path = repo_path.join("main.typ");
    if main_path.exists() {
        // Round-trip: read existing summary, patch cover, rewrite the directive.
        let config = fx_renderer::fx_render_config();
        let repo = repo_path.to_path_buf();
        let current = tokio::task::spawn_blocking(move || {
            fx_renderer::extract_typst_series_summary(&repo, &config)
        }).await.ok().flatten().unwrap_or_default();
        let mut updated = current;
        updated.cover = cover;
        let source = tokio::fs::read_to_string(&main_path).await.unwrap_or_default();
        let new_source = fx_renderer::upsert_typst_series_summary(&source, &updated);
        if let Err(e) = tokio::fs::write(&main_path, new_source).await {
            tracing::warn!("update typst series summary failed: {e}");
        }
    } else if let Err(e) = fx_core::meta::set_series_meta_cover(repo_path, cover) {
        tracing::warn!("update series meta cover failed: {e}");
    }
}

/// Write the cover file into the article/series blob cache. The blob cache
/// is the single filesystem source of truth for locally-authored content in
/// the blob storage model. Cover blobs are appended to the article record's
/// top-level `files[]` via `append_file_to_article_record` on upload; the
/// on-disk copy here just keeps the renderer happy.
async fn write_cover_to_repo(
    state: &AppState,
    repo_uri: &str,
    _did: &str,
    data: &[u8],
    ext: &str,
    is_series: bool,
) -> Result<(), AppError> {
    let node_id = fx_core::util::uri_to_node_id(repo_uri);
    let repo_path = state.blob_cache_path.join(&node_id);
    tokio::fs::create_dir_all(&repo_path).await?;

    purge_other_exts(&repo_path, ext).await;
    let cover_path = repo_path.join(format!("{COVER_STEM}.{ext}"));
    tokio::fs::write(&cover_path, data).await?;

    if is_series {
        persist_series_cover(&repo_path, Some(format!("{COVER_STEM}.{ext}"))).await;
    }
    Ok(())
}

/// Write the cover selection into the article source's format-native metadata
/// slot so it travels with the source files. Returns whether the file changed.
///
/// * markdown: `cover:` in YAML frontmatter at top of `content.md`
/// * typst:    `#metadata(("cover": "...")) <nbt-article>` at top of `content.typ`
/// * html:     `<meta name="nightboat:cover" content="...">` in `<head>` of `content.html`
async fn update_article_cover_meta(
    pool: &sqlx::PgPool,
    repo_path: &std::path::Path,
    article_uri: &str,
    cover: Option<String>,
) -> Result<bool, AppError> {
    let format: Option<String> = sqlx::query_scalar(
        "SELECT content_format::text FROM article_localizations WHERE at_uri = $1",
    )
    .bind(article_uri)
    .fetch_optional(pool)
    .await?;

    let (filename, rewrite): (&str, fn(&str, Option<String>) -> String) = match format.as_deref() {
        Some("markdown") => ("content.md",   fx_core::meta::rewrite_markdown_cover),
        Some("typst")    => ("content.typ",  rewrite_typst_article_cover),
        Some("html")     => ("content.html", rewrite_html_article_cover),
        _ => return Ok(false),
    };

    let content_path = repo_path.join(filename);
    let src = match tokio::fs::read_to_string(&content_path).await {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    let new_src = rewrite(&src, cover);
    if new_src == src {
        return Ok(false);
    }
    tokio::fs::write(&content_path, new_src).await?;
    Ok(true)
}

/// Upsert the `#metadata(("cover": "...")) <nbt-article>` line at the top of a
/// typst source. Any existing line carrying that label is removed first so we
/// never leave two covers behind. A None cover means "remove it".
fn rewrite_typst_article_cover(source: &str, cover: Option<String>) -> String {
    let stripped = TYPST_COVER_RE.replace_all(source, "").to_string();
    match cover {
        Some(c) => {
            let escaped = c.replace('\\', r"\\").replace('"', r#"\""#);
            let leading_newline = if stripped.starts_with('\n') { "" } else { "\n" };
            format!("#metadata((cover: \"{escaped}\")) <nbt-article>\n{leading_newline}{stripped}")
        }
        None => stripped,
    }
}

/// Parse `<meta name="nightboat:cover" content="...">` out of an HTML source.
pub(super) fn html_cover_from_meta(source: &str) -> Option<String> {
    let caps = HTML_COVER_META_RE.captures(source)?;
    let raw = caps.get(1)?.as_str();
    let decoded = raw
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    Some(decoded)
}

/// Upsert `<meta name="nightboat:cover" content="...">` in an HTML source's
/// head. Idempotent; None removes the tag. Falls back to prepending the tag
/// when the document has no recognizable `<head>`.
fn rewrite_html_article_cover(source: &str, cover: Option<String>) -> String {
    let stripped = HTML_COVER_META_STRIP_RE.replace_all(source, "").to_string();
    let Some(c) = cover else { return stripped; };
    let escaped = c
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let tag = format!("<meta name=\"nightboat:cover\" content=\"{escaped}\">\n");
    // Insert right after <head> if present; else just prepend.
    if let Some(m) = HTML_HEAD_OPEN_RE.find(&stripped) {
        let (a, b) = stripped.split_at(m.end());
        let needs_newline = if b.starts_with('\n') { "" } else { "\n" };
        format!("{a}{needs_newline}{tag}{b}")
    } else {
        format!("{tag}{stripped}")
    }
}

/// After the article's source-format cover metadata (markdown frontmatter,
/// typst directive, etc.) is updated on disk, the upload path already
/// `append_file_to_article_record`s the new cover blob into the record's
/// `files[]`, and the source file change is staged into blob_cache. The
/// "set cover by reference" path points at an already-uploaded file so the
/// record is in sync without an extra write. No-op intentionally.
async fn record_article_cover_meta(_state: &AppState, _repo_uri: &str, _did: &str) {}

/// Delete every cover.{ext} in the blob cache dir.
async fn remove_cover_from_repo(
    state: &AppState,
    repo_uri: &str,
    _did: &str,
    is_series: bool,
) {
    let node_id = fx_core::util::uri_to_node_id(repo_uri);
    let repo_path = state.blob_cache_path.join(&node_id);
    if is_series {
        persist_series_cover(&repo_path, None).await;
    }
    for ext in UPLOAD_EXTENSIONS {
        let _ = tokio::fs::remove_file(repo_path.join(format!("{COVER_STEM}.{ext}"))).await;
    }
}

#[derive(serde::Deserialize)]
pub struct UriQuery { pub uri: String }

#[derive(serde::Deserialize)]
pub struct IdQuery { pub id: String }

#[derive(serde::Deserialize)]
pub struct CoverReference { pub file: String }

fn mime_for_image_ext(ext: &str) -> &'static str {
    match ext {
        "png"          => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif"          => "image/gif",
        "webp"         => "image/webp",
        "svg"          => "image/svg+xml",
        "avif"         => "image/avif",
        _              => "application/octet-stream",
    }
}

pub async fn upload_article_cover(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<UriQuery>,
    multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let owner_did = article_service::get_article_owner(&state.pool, &q.uri).await?;
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the author can set a cover".into())));
    }

    let node_id = uri_to_node_id(&q.uri);
    let (data, ext) = read_multipart(multipart).await?;
    let cover_file = format!("{COVER_STEM}.{ext}");
    let mime = mime_for_image_ext(&ext);

    // 1. Upload/synthesize as a PDS blob under this article's author.
    let blob = crate::auth::upload_or_local_blob(
        &state, &user.token, &user.did, data.clone(), mime,
    ).await;

    // 2. Materialize the cover byte-for-byte into blob_cache so the
    //    `/api/covers/a-{node_id}` image endpoint can serve it directly.
    //    This also handles purging stale covers under a different extension.
    let repo_path = state.blob_cache_path.join(&node_id);
    tokio::fs::create_dir_all(&repo_path).await?;
    purge_other_exts(&repo_path, &ext).await;
    tokio::fs::write(repo_path.join(&cover_file), &data).await?;

    // 3. Append the cover to the article's blob bundle (content_manifest
    //    + PDS at.nightbo.work record).
    super::articles::append_file_to_article_manifest(
        &state, &q.uri, &cover_file, &blob, data.len(), mime,
    ).await?;
    super::articles::append_file_to_article_record(
        &state, &user.token, &user.did, &q.uri, &cover_file, &blob, data.len(), mime,
    ).await?;

    // 4. Update the DB cover pointer.
    let cover_url = format!("/api/covers/a-{node_id}");
    sqlx::query(
        "UPDATE articles a SET cover_url = $1, cover_file = $2 \
         FROM article_localizations l \
         WHERE l.at_uri = $3 AND a.repo_uri = l.repo_uri AND a.source_path = l.source_path",
    )
        .bind(&cover_url).bind(&cover_file).bind(&q.uri).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url, "cover_file": cover_file })))
}

pub async fn remove_article_cover(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<UriQuery>,
) -> ApiResult<StatusCode> {
    let owner_did = article_service::get_article_owner(&state.pool, &q.uri).await?;
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the author can remove the cover".into())));
    }
    remove_cover_from_repo(&state, &q.uri, &user.did, false).await;
    let repo_path = state.blob_cache_path.join(fx_core::util::uri_to_node_id(&q.uri));
    if update_article_cover_meta(&state.pool, &repo_path, &q.uri, None).await.unwrap_or(false) {
        record_article_cover_meta(&state, &q.uri, &user.did).await;
    }
    sqlx::query(
        "UPDATE articles a SET cover_url = NULL, cover_file = NULL \
         FROM article_localizations l \
         WHERE l.at_uri = $1 AND a.repo_uri = l.repo_uri AND a.source_path = l.source_path",
    )
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
    let owner_did = article_service::get_article_owner(&state.pool, &q.uri).await?;
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
    let repo_path = state.blob_cache_path.join(&node_id);
    if !tokio::fs::try_exists(repo_path.join(file)).await.unwrap_or(false) {
        return Err(AppError(fx_core::Error::BadRequest(
            format!("cover file not found in repo: {file}"),
        )));
    }
    let cover_url = format!("/api/covers/a-{node_id}");
    if update_article_cover_meta(&state.pool, &repo_path, uri, Some(file.to_string())).await.unwrap_or(false) {
        if let Ok(d) = article_service::get_article_owner(&state.pool, uri).await {
            record_article_cover_meta(state, uri, &d).await;
        }
    }
    sqlx::query(
        "UPDATE articles a SET cover_url = $1, cover_file = $2 \
         FROM article_localizations l \
         WHERE l.at_uri = $3 AND a.repo_uri = l.repo_uri AND a.source_path = l.source_path",
    )
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
    article_service::get_article_owner(&state.pool, &q.uri).await?;
    set_article_cover_reference_inner(&state, &q.uri, &body.file).await
}

pub async fn upload_series_cover(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<IdQuery>,
    multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let owner_did = series_service::get_series_owner(&state.pool, &q.id).await?;
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the creator can set a cover".into())));
    }

    let series_repo_uri = series_service::series_repo_uri(&state.pool, &q.id).await?;
    let node_id = uri_to_node_id(&series_repo_uri);
    let (data, ext) = read_multipart(multipart).await?;
    let cover_file = format!("{COVER_STEM}.{ext}");
    let mime = mime_for_image_ext(&ext);

    // Upload/synthesize as a blob, materialize to blob_cache for local
    // serving, then append the cover to the series PDS record's files[].
    let blob = crate::auth::upload_or_local_blob(
        &state, &user.token, &user.did, data.clone(), mime,
    ).await;
    let repo_path = state.blob_cache_path.join(&node_id);
    tokio::fs::create_dir_all(&repo_path).await?;
    purge_other_exts(&repo_path, &ext).await;
    tokio::fs::write(repo_path.join(&cover_file), &data).await?;
    super::articles::append_file_to_series_record(
        &state, &user.token, &user.did, &q.id, &cover_file, &blob, mime,
    ).await;

    let cover_url = format!("/api/covers/s-{node_id}");
    sqlx::query("UPDATE series SET cover_url = $1, cover_file = $2 WHERE id = $3")
        .bind(&cover_url).bind(&cover_file).bind(&q.id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url, "cover_file": cover_file })))
}

pub async fn remove_series_cover(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    axum::extract::Query(q): axum::extract::Query<IdQuery>,
) -> ApiResult<StatusCode> {
    let owner_did = series_service::get_series_owner(&state.pool, &q.id).await?;
    if owner_did != user.did {
        return Err(AppError(fx_core::Error::BadRequest("only the creator can remove the cover".into())));
    }
    let series_repo_uri = series_service::series_repo_uri(&state.pool, &q.id).await?;
    remove_cover_from_repo(&state, &series_repo_uri, &user.did, true).await;
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
    let owner_did = series_service::get_series_owner(&state.pool, &q.id).await?;
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
    let series_repo_uri = series_service::series_repo_uri(&state.pool, series_id).await?;
    let node_id = uri_to_node_id(&series_repo_uri);

    let repo_path = state.blob_cache_path.join(&node_id);
    tokio::fs::create_dir_all(&repo_path).await
        .map_err(|e| AppError(fx_core::Error::Internal(format!("create blob cache: {e}"))))?;
    if !tokio::fs::try_exists(repo_path.join(file)).await.unwrap_or(false) {
        return Err(AppError(fx_core::Error::BadRequest(
            format!("cover file not found in repo: {file}"),
        )));
    }

    // Persist the cover selection into the series' metadata slot when/if we
    // reintroduce cross-chapter series metadata files.
    persist_series_cover(&repo_path, Some(file.to_string())).await;

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
    let owner_did = series_service::get_series_owner(&state.pool, &q.id).await?;
    let series_repo_uri = series_service::series_repo_uri(&state.pool, &q.id).await?;
    let node_id = uri_to_node_id(&series_repo_uri);
    let (data, ext) = read_multipart(multipart).await?;
    let cover_file = format!("{COVER_STEM}.{ext}");
    let mime = mime_for_image_ext(&ext);

    // Blob upload: admin has no user PDS token, so for real-PDS users the
    // upload silently no-ops and only the local blob_cache gets populated.
    // For did:local:* users it synthesizes a local CID.
    let blob = crate::auth::upload_or_local_blob(
        &state, "", &owner_did, data.clone(), mime,
    ).await;
    let repo_path = state.blob_cache_path.join(&node_id);
    tokio::fs::create_dir_all(&repo_path).await?;
    purge_other_exts(&repo_path, &ext).await;
    tokio::fs::write(repo_path.join(&cover_file), &data).await?;
    super::articles::append_file_to_series_record(
        &state, "", &owner_did, &q.id, &cover_file, &blob, mime,
    ).await;

    let cover_url = format!("/api/covers/s-{node_id}");
    sqlx::query("UPDATE series SET cover_url = $1, cover_file = $2 WHERE id = $3")
        .bind(&cover_url).bind(&cover_file).bind(&q.id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "cover_url": cover_url, "cover_file": cover_file })))
}

pub async fn admin_remove_series_cover(
    State(state): State<AppState>,
    _admin: AdminAuth,
    axum::extract::Query(q): axum::extract::Query<IdQuery>,
) -> ApiResult<StatusCode> {
    let owner_did = series_service::get_series_owner(&state.pool, &q.id).await?;
    let series_repo_uri = series_service::series_repo_uri(&state.pool, &q.id).await?;
    remove_cover_from_repo(&state, &series_repo_uri, &owner_did, true).await;
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
    series_service::get_series_owner(&state.pool, &q.id).await?;
    set_series_cover_reference_inner(&state, &q.id, &body.file).await
}
