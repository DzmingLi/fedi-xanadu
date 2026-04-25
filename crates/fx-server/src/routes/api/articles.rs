use axum::{
    Json,
    extract::{Multipart, Query, State},
    http::StatusCode,
};
use fx_core::content::{ContentFormat, ContentKind};
use fx_core::models::*;
use fx_core::region::default_visibility;
use fx_core::services::{article_service, authorship_service, collaboration_service, notification_service, series_service, version_service};
use fx_core::validation::validate_create_article;

use crate::error::{AppError, ApiResult, require_owner};
use crate::state::AppState;
use crate::auth::{WriteAuth, pds_create_record, pds_delete_record};
use fx_core::util::{content_hash, tid, uri_to_node_id, now_rfc3339};
use super::UriQuery;

/// Resolve a collaborator identifier (DID or atproto handle) to a DID.
/// Passes `did:` URIs through unchanged; otherwise calls
/// `AtClient::resolve_handle` and returns a `BadRequest` on failure so the
/// caller hears the exact reason (unknown handle, network, etc.). Stays in
/// the server crate because it depends on the server-owned `AtClient`.
pub(super) async fn resolve_identifier(state: &AppState, identifier: &str) -> Result<String, AppError> {
    let s = identifier.trim();
    if s.is_empty() {
        return Err(AppError(fx_core::Error::BadRequest("identifier is empty".into())));
    }
    if s.starts_with("did:") {
        return Ok(s.to_string());
    }
    let (did, _pds) = state.at_client.resolve_handle(s).await
        .map_err(|e| AppError(fx_core::Error::BadRequest(format!("cannot resolve handle '{s}': {e}"))))?;
    Ok(did)
}

/// Pre-compiled regex for extracting anchor IDs from HTML.
static ANCHOR_ID_RE: std::sync::LazyLock<regex_lite::Regex> =
    std::sync::LazyLock::new(|| regex_lite::Regex::new(r##"id="([^"]+)""##).unwrap());

/// Pre-compiled regex for rewriting cross-chapter href links.
static LINK_HREF_RE: std::sync::LazyLock<regex_lite::Regex> =
    std::sync::LazyLock::new(|| regex_lite::Regex::new(r##"href="#([^"]+)""##).unwrap());

#[derive(serde::Deserialize)]
pub struct ListArticlesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_articles(
    State(state): State<AppState>,
    Query(q): Query<ListArticlesQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let offset = q.offset.unwrap_or(0).max(0);
    let articles = article_service::list_articles(&state.pool, state.instance_mode, limit, offset).await?;
    Ok(Json(articles))
}

pub async fn get_article(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Article>> {
    let article = article_service::get_article(&state.pool, state.instance_mode, &uri).await?;
    Ok(Json(article))
}

/// Resolve an article by `handle`/`slug[.lang]`.
///
/// - `:slug_maybe_lang` is either `{rkey}` (bare) or `{rkey}.{lang}`
///   (language-specific).
/// - Bare form: picks a localization by `Accept-Language`, then 302-redirects
///   to the explicit `.{lang}` URL (so the lang is always reflected in the
///   URL — shareable + cacheable). Per the no-fallback rule, if no available
///   localization matches the user's preferences we still pick the source
///   language explicitly rather than silently falling back.
/// - Explicit form: returns the JSON of that specific localization or 404.
pub async fn get_article_by_slug(
    State(state): State<AppState>,
    axum::extract::Path((handle, slug_maybe_lang)): axum::extract::Path<(String, String)>,
    headers: axum::http::HeaderMap,
) -> Result<axum::response::Response, AppError> {
    use axum::response::IntoResponse;

    let did = fx_core::services::platform_user_service::local_did(&handle);
    let (slug, requested_lang) = split_slug_lang(&slug_maybe_lang);
    let repo_uri = format!("at://{did}/{lex}/{slug}", lex = fx_atproto::lexicon::WORK);

    if let Some(lang) = requested_lang {
        // Explicit lang: direct lookup by (repo_uri, source_path, lang).
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT at_uri FROM article_localizations \
             WHERE repo_uri = $1 AND lang = $2 \
             LIMIT 1",
        )
        .bind(&repo_uri)
        .bind(lang)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError(e.into()))?;
        let Some((at_uri,)) = row else {
            return Err(AppError(fx_core::Error::NotFound {
                entity: "article",
                id: format!("{handle}/{slug_maybe_lang}"),
            }));
        };
        // Use the any-lang view so translations (file_path ≠ source_path)
        // resolve correctly; `get_article` pins to the source-lang row.
        let article = article_service::get_article_any_lang(&state.pool, &at_uri).await?;
        return Ok(Json(article).into_response());
    }

    // Bare slug: pick a lang via Accept-Language negotiation.
    let available: Vec<(String,)> = sqlx::query_as(
        "SELECT lang FROM article_localizations WHERE repo_uri = $1 ORDER BY lang",
    )
    .bind(&repo_uri)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| AppError(e.into()))?;
    if available.is_empty() {
        return Err(AppError(fx_core::Error::NotFound {
            entity: "article",
            id: format!("{handle}/{slug}"),
        }));
    }
    let available: Vec<String> = available.into_iter().map(|(l,)| l).collect();
    let chosen = pick_lang(&headers, &available);

    let location = format!("/@{handle}/{slug}.{chosen}");
    Ok(axum::response::Redirect::to(&location).into_response())
}

fn split_slug_lang(s: &str) -> (&str, Option<&str>) {
    match s.rsplit_once('.') {
        Some((stem, lang)) if !lang.is_empty() && looks_like_lang_tag(lang) => (stem, Some(lang)),
        _ => (s, None),
    }
}

/// Heuristic: BCP 47 lang tags are letters + hyphens (e.g. "en", "zh-CN").
/// Rejecting "md"/"typ" etc. so `foo.md`-like slugs don't trigger the
/// lang-split path by accident.
fn looks_like_lang_tag(s: &str) -> bool {
    s.len() >= 2
        && s.contains('-')
            .then_some(true)
            .unwrap_or_else(|| s.len() == 2 || s.len() == 3)
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Pick the best-matching language from `available` given the request's
/// `Accept-Language` header. Falls back to the alphabetically first
/// available (typically the source lang) when nothing matches — explicit,
/// not silent: the caller still ends up on a URL that names the chosen lang.
fn pick_lang<'a>(headers: &axum::http::HeaderMap, available: &'a [String]) -> &'a str {
    let accept = headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    for entry in accept.split(',') {
        let tag = entry.split(';').next().unwrap_or("").trim();
        if tag.is_empty() {
            continue;
        }
        // Exact match
        if let Some(hit) = available.iter().find(|l| l.eq_ignore_ascii_case(tag)) {
            return hit;
        }
        // Primary-tag match (accept "zh" → "zh-CN")
        let primary = tag.split('-').next().unwrap_or(tag);
        if let Some(hit) = available
            .iter()
            .find(|l| l.split('-').next().map(|p| p.eq_ignore_ascii_case(primary)).unwrap_or(false))
        {
            return hit;
        }
    }
    available.first().map(String::as_str).unwrap_or("")
}

pub async fn get_article_content(
    State(state): State<AppState>,
    crate::auth::MaybeAuth(user): crate::auth::MaybeAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<ArticleContent>> {
    let has_access = article_service::check_content_access(
        &state.pool, &uri, user.as_ref().map(|u| u.did.as_str()),
    ).await?;
    if !has_access {
        return Err(AppError(fx_core::Error::Forbidden { action: "view restricted article" }));
    }

    let format = article_service::get_content_format(&state.pool, &uri).await?;
    Ok(Json(resolve_article_content(&state, &uri, &format).await?))
}

/// Shared content resolution. Dispatches to the right backend based on how
/// the article's bytes are stored:
///   1. **Heading slice** — a pre-split chapter whose HTML sits in the parent
///      series' compile cache.
///   2. **Thought** — short note stored inline in `article_versions`; no pijul.
///   3. **Standard** — source file in a pijul repo (standalone or series
///      chapter); renders on demand with a cache-aside layer.
async fn resolve_article_content(
    state: &AppState,
    uri: &str,
    format: &str,
) -> Result<ArticleContent, AppError> {
    if let Some(content) = resolve_heading_slice(state, uri).await? {
        return Ok(content);
    }
    if let Some(content) = resolve_thought(state, uri, format).await? {
        return Ok(content);
    }
    resolve_standard_content(state, uri, format).await
}

/// Compile-generated chapter whose HTML was written to a series' compile
/// cache. The pijul-backed series compile path has been removed; heading
/// slicing is pending re-implementation on top of per-chapter blob articles,
/// so this returns None for now.
async fn resolve_heading_slice(
    _state: &AppState,
    _uri: &str,
) -> Result<Option<ArticleContent>, AppError> {
    Ok(None)
}

/// Thought: source + optional cached HTML live in the latest `article_versions`
/// row (no pijul repo). Returns None for non-thought kinds.
async fn resolve_thought(
    state: &AppState,
    uri: &str,
    format: &str,
) -> Result<Option<ArticleContent>, AppError> {
    let kind: Option<String> = sqlx::query_scalar(
        "SELECT a.kind::TEXT FROM articles a \
         JOIN article_localizations l \
           ON a.repo_uri = l.repo_uri AND a.source_path = l.source_path \
         WHERE l.at_uri = $1",
    )
        .bind(uri).fetch_optional(&state.pool).await?;
    if kind.as_deref() != Some("thought") { return Ok(None); }

    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT source_text, rendered_html FROM article_versions WHERE article_uri = $1 ORDER BY created_at DESC LIMIT 1"
    ).bind(uri).fetch_optional(&state.pool).await?;

    let (source, cached_html) = row
        .ok_or(AppError(fx_core::Error::NotFound { entity: "content", id: uri.to_string() }))?;

    let html = if let Some(h) = cached_html {
        h
    } else if format == "html" {
        source.clone()
    } else {
        let tmp = std::env::temp_dir().join(format!("nb-thought-{}", tid()));
        let _ = tokio::fs::create_dir_all(&tmp).await;
        let rendered = render_content(format, &source, &tmp)?;
        let _ = tokio::fs::remove_dir_all(&tmp).await;
        let _ = sqlx::query("UPDATE article_versions SET rendered_html = $1 WHERE article_uri = $2 AND rendered_html IS NULL")
            .bind(&rendered).bind(uri).execute(&state.pool).await;
        rendered
    };
    Ok(Some(ArticleContent { source, html }))
}

/// Standalone article or series chapter: source in a pijul repo, HTML
/// rendered on demand with a file-backed cache-aside.
async fn resolve_standard_content(
    state: &AppState,
    uri: &str,
    format: &str,
) -> Result<ArticleContent, AppError> {
    let loc = resolve_location(state, uri, format).await
        .ok_or(AppError(fx_core::Error::NotFound { entity: "content", id: uri.to_string() }))?;

    let source = tokio::fs::read_to_string(&loc.content_path).await
        .map_err(|_| AppError(fx_core::Error::NotFound { entity: "content", id: uri.to_string() }))?;

    let html = if format == "html" {
        source.clone()
    } else if let Some(series_html) = try_series_render(state, uri, format).await {
        series_html
    } else if is_cached_fresh(&loc.cache_path, &loc.content_path).await {
        tokio::fs::read_to_string(&loc.cache_path).await?
    } else {
        let rendered = render_content(format, &source, &loc.repo_path)?;
        if let Some(parent) = loc.cache_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&loc.cache_path, &rendered).await;
        rendered
    };

    let html = loc.rewrite_html_urls(&html, uri);
    Ok(ArticleContent { source, html })
}

/// Shared regex for both relative-URL rewriters (compiled once at startup).
static REL_URL_RE: std::sync::LazyLock<regex_lite::Regex> =
    std::sync::LazyLock::new(|| regex_lite::Regex::new(r#"(src|href)="([^"]*?)""#).expect("rel url regex"));

fn is_absolute_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
        || url.starts_with('/') || url.starts_with('#')
        || url.starts_with("data:") || url.starts_with("mailto:")
}

/// Rewrite relative `src` and `href` attributes in HTML to point to a base URL.
/// Only rewrites paths that don't start with `/`, `http://`, `https://`, or `#`.
fn rewrite_relative_urls(html: &str, base_url: &str) -> String {
    REL_URL_RE.replace_all(html, |caps: &regex_lite::Captures| {
        let attr = &caps[1];
        let url = &caps[2];
        if is_absolute_url(url) {
            caps[0].to_string()
        } else {
            format!("{attr}=\"{base_url}/{url}\"")
        }
    }).to_string()
}

/// Rewrite relative URLs for standalone articles: `src="foo.png"` → `src="/api/articles/image?uri=...&name=foo.png"`
fn rewrite_relative_urls_with_query(html: &str, endpoint: &str, encoded_uri: &str) -> String {
    REL_URL_RE.replace_all(html, |caps: &regex_lite::Captures| {
        let attr = &caps[1];
        let url = &caps[2];
        if is_absolute_url(url) {
            caps[0].to_string()
        } else {
            format!("{attr}=\"{endpoint}?uri={encoded_uri}&name={}\"", urlencoding::encode(url))
        }
    }).to_string()
}

/// Resolved location of an article's content in the pijul store.
/// Eliminates series-vs-standalone branching throughout the codebase.
pub(super) struct ArticleLocation {
    /// Pijul repo node_id (used for pijul operations).
    pub node_id: String,
    /// Filesystem path to the pijul repo root.
    pub repo_path: std::path::PathBuf,
    /// Path to the source file within the repo.
    pub content_path: std::path::PathBuf,
    /// Path to the cached HTML file.
    pub cache_path: std::path::PathBuf,
    /// Series ID, if this article belongs to a series.
    pub series_id: Option<String>,
    /// Chapter ID within the series (TID from the article URI). Carried so
    /// future callers that need the split-output anchor can recover it from
    /// the location struct without re-parsing the URI.
    #[allow(dead_code)]
    pub chapter_id: Option<String>,
}

impl ArticleLocation {
    /// Is this a series chapter?
    pub fn is_series(&self) -> bool { self.series_id.is_some() }

    /// Build the resource URL for rewriting relative paths in HTML.
    pub fn rewrite_html_urls(&self, html: &str, article_uri: &str) -> String {
        if let Some(ref sid) = self.series_id {
            let chapter_dir = self.content_path.parent().unwrap_or(&self.repo_path);
            let rel_dir = chapter_dir.strip_prefix(&self.repo_path)
                .unwrap_or(chapter_dir).to_string_lossy();
            let base_url = format!("/api/series/{sid}/res/{}", rel_dir.trim_start_matches('/'));
            rewrite_relative_urls(html, &base_url)
        } else {
            let encoded_uri = urlencoding::encode(article_uri);
            rewrite_relative_urls_with_query(html, "/api/articles/image", &encoded_uri)
        }
    }

    /// Invalidate the HTML cache for this article.
    pub async fn invalidate_cache(&self) {
        let _ = tokio::fs::remove_file(&self.cache_path).await;
        if self.is_series() {
            // Also invalidate series-wide cache
            let _ = tokio::fs::remove_file(self.repo_path.join("cache").join("series.cache")).await;
        }
    }
}

/// Per-file entry inside `articles.content_manifest` JSONB for blob-backed articles.
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub(super) struct BlobManifestFile {
    pub path: String,
    pub cid: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub mime: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub(super) struct BlobManifest {
    pub entry: String,
    pub files: Vec<BlobManifestFile>,
    /// PDS endpoint to fetch blobs from. Stored in the manifest so image /
    /// render requests don't need to resolve the DID doc on every hit.
    pub pds_url: String,
}

/// Materialize blob-backed source files under `{blob_cache}/{node_id}/` so the
/// renderer sees the same layout as a pijul working dir. Idempotent: existing
/// files with matching size are left alone. Returns the scratch repo path.
pub(super) async fn ensure_blob_materialized(
    state: &AppState,
    did: &str,
    manifest: &BlobManifest,
    repo_path: &std::path::Path,
) -> Result<(), AppError> {
    tokio::fs::create_dir_all(repo_path).await
        .map_err(|e| AppError(fx_core::Error::Internal(format!("create blob cache dir: {e}"))))?;
    for file in &manifest.files {
        if file.path.is_empty() || file.path.contains("..") || file.path.starts_with('/') {
            return Err(AppError(fx_core::Error::Internal(format!(
                "invalid manifest path: {}", file.path
            ))));
        }
        let dest = repo_path.join(&file.path);
        if let Ok(meta) = tokio::fs::metadata(&dest).await {
            if meta.is_file() && (file.size == 0 || meta.len() == file.size) {
                continue;
            }
        }
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| AppError(fx_core::Error::Internal(format!("create parent dir: {e}"))))?;
        }
        let bytes = state.at_client.get_blob(&manifest.pds_url, did, &file.cid).await
            .map_err(|e| AppError(fx_core::Error::Internal(format!("get_blob {}: {e}", file.cid))))?;
        let tmp = dest.with_extension(format!(
            "{}.part",
            dest.extension().and_then(|e| e.to_str()).unwrap_or("")
        ));
        tokio::fs::write(&tmp, &bytes).await
            .map_err(|e| AppError(fx_core::Error::Internal(format!("write blob: {e}"))))?;
        tokio::fs::rename(&tmp, &dest).await
            .map_err(|e| AppError(fx_core::Error::Internal(format!("rename blob: {e}"))))?;
    }
    Ok(())
}

/// Load a blob-backed article's manifest if storage = 'blob'. Returns None for
/// pijul-backed (or missing) articles.
pub(super) async fn load_blob_manifest(
    pool: &sqlx::PgPool,
    article_uri: &str,
) -> Option<BlobManifest> {
    let row: Option<(String, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT content_storage, content_manifest FROM article_localizations WHERE at_uri = $1",
    )
    .bind(article_uri)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    let (storage, manifest) = row?;
    if storage != "blob" { return None; }
    serde_json::from_value::<BlobManifest>(manifest?).ok()
}

/// Extract the owning DID from an at-uri (`at://DID/...`).
fn uri_did(article_uri: &str) -> Option<&str> {
    article_uri.strip_prefix("at://")?.split('/').next()
}

/// Synthetic URI used to address series chapters without minting a PDS
/// record: `nightboat-chapter://{series_id}/{source_path_urlencoded}`. The
/// source_path is URL-encoded so nested paths (e.g. `chapters/01.typ`) round
/// trip cleanly through code that splits on `/`.
pub(super) const CHAPTER_URI_SCHEME: &str = "nightboat-chapter://";

pub(super) fn build_chapter_uri(series_id: &str, source_path: &str) -> String {
    format!("{CHAPTER_URI_SCHEME}{series_id}/{}", urlencoding::encode(source_path))
}

/// Parse a `nightboat-chapter://{series_id}/{source_path}` URI. Returns
/// `None` when the URI is not a chapter URI.
pub(super) fn parse_chapter_uri(uri: &str) -> Option<(String, String)> {
    let rest = uri.strip_prefix(CHAPTER_URI_SCHEME)?;
    let (series_id, encoded_path) = rest.split_once('/')?;
    let source_path = urlencoding::decode(encoded_path).ok()?.into_owned();
    Some((series_id.to_string(), source_path))
}

/// Resolve a chapter's on-disk location — source file under the SERIES's
/// shared blob_cache dir, not a per-chapter dir. Separate path from the
/// standalone blob resolver because chapters don't have a per-chapter
/// manifest/at_uri to look up.
async fn resolve_chapter_location(
    state: &AppState,
    series_id: &str,
    source_path: &str,
) -> Option<ArticleLocation> {
    // Look up the chapter row; also gives us the series repo_uri.
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT sa.repo_uri, sa.heading_anchor FROM series_articles sa \
         WHERE sa.series_id = $1 AND sa.source_path = $2",
    )
    .bind(series_id)
    .bind(source_path)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    let (series_repo_uri, anchor) = row?;

    let node_id = uri_to_node_id(&series_repo_uri);
    let repo_path = state.blob_cache_path.join(&node_id);
    let content_path = repo_path.join(source_path);
    let anchor_stem = anchor
        .clone()
        .or_else(|| std::path::Path::new(source_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(String::from))
        .unwrap_or_else(|| "chapter".to_string());
    let cache_path = repo_path.join("cache").join(format!("{anchor_stem}.html"));
    Some(ArticleLocation {
        node_id,
        repo_path,
        content_path,
        cache_path,
        series_id: Some(series_id.to_string()),
        chapter_id: anchor,
    })
}

/// Resolve the on-disk location of an article's source tree. Dispatches:
///   - `nightboat-chapter://{series_id}/{source_path}` → series's shared
///     blob_cache; chapter source lives at its `source_path` within that
///     dir; rendered HTML cache lives under `cache/{anchor}.html`.
///   - `at://...` → standalone article; source + cache under
///     `blob_cache_path/{uri_to_node_id(at_uri)}/`.
pub(super) async fn resolve_location(
    state: &AppState,
    article_uri: &str,
    format: &str,
) -> Option<ArticleLocation> {
    if let Some((series_id, source_path)) = parse_chapter_uri(article_uri) {
        return resolve_chapter_location(state, &series_id, &source_path).await;
    }

    let src_ext = fx_renderer::format_extension(format);
    let manifest = load_blob_manifest(&state.pool, article_uri).await?;
    let did = uri_did(article_uri)?;
    let node_id = uri_to_node_id(article_uri);
    let repo_path = state.blob_cache_path.join(&node_id);
    if let Err(e) = ensure_blob_materialized(state, did, &manifest, &repo_path).await {
        tracing::warn!("blob materialize failed for {article_uri}: {e:?}");
        return None;
    }
    let content_path = repo_path.join(format!("content.{src_ext}"));
    let cache_path = repo_path.join("content.html");
    Some(ArticleLocation {
        node_id,
        repo_path,
        content_path,
        cache_path,
        series_id: None,
        chapter_id: None,
    })
}


/// Series-aware rendering (typst whole-document compile, anchor rewriting) is
/// pending re-implementation on top of per-chapter blob articles: every
/// chapter now owns its own at.nightbo.work record + file-tree, so
/// cross-chapter compilation has to re-assemble the virtual document from
/// multiple blob bundles. Until that's designed, every chapter renders
/// individually through the standard content path.
async fn try_series_render(
    _state: &crate::state::AppState,
    _article_uri: &str,
    _format: &str,
) -> Option<String> {
    None
}

async fn is_cached_fresh(cache: &std::path::Path, source: &std::path::Path) -> bool {
    let (cache_meta, source_meta) = tokio::join!(
        tokio::fs::metadata(cache),
        tokio::fs::metadata(source),
    );
    let (Ok(c), Ok(s)) = (cache_meta, source_meta) else { return false };
    let (Ok(c_mod), Ok(s_mod)) = (c.modified(), s.modified()) else { return false };
    c_mod >= s_mod
}

pub(super) fn render_content(format: &str, source: &str, repo_path: &std::path::Path) -> Result<String, AppError> {
    let config = fx_renderer::fx_render_config();
    fx_renderer::render_to_html_with_config(format, source, repo_path, &config)
        .map_err(|e| { tracing::warn!("render error: {e}"); AppError(fx_core::Error::Render(e.to_string())) })
}

#[derive(serde::Deserialize)]
pub struct PrereqsQuery {
    pub uri: String,
    #[serde(default)]
    pub locale: Option<String>,
}

pub async fn get_article_prereqs(
    State(state): State<AppState>,
    Query(q): Query<PrereqsQuery>,
) -> ApiResult<Json<Vec<ArticlePrereqRow>>> {
    let locale = q.locale.as_deref().unwrap_or("en");
    let prereqs = article_service::get_article_prereqs(&state.pool, &q.uri, locale).await?;
    Ok(Json(prereqs))
}

pub async fn get_article_forks(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<ForkWithTitle>>> {
    let forks = article_service::get_article_forks(&state.pool, &uri).await?;
    Ok(Json(forks))
}

/// Result of publishing article content: repo path + resolved summary.
pub(super) struct PublishResult {
    /// Repo root where this article was written. Currently only read by
    /// pijul-record callsites; kept public to avoid churn at those sites.
    #[allow(dead_code)]
    pub repo_path: std::path::PathBuf,
    /// Final summary source (author-supplied — auto extraction is roadmapped
    /// via LLM, not simple truncation).
    pub summary_source: String,
    /// Summary rendered to inline-only HTML, cached in DB for list views and
    /// the lead block at the top of the article body.
    pub summary_html: String,
    /// Blob manifest when content was published via PDS blobs (the default
    /// `use_pijul=false` path). `None` for pijul-backed publishes. Callers
    /// thread this into `article_service::create_article` so the
    /// localization row is created with `content_storage='blob'` and the
    /// JSONB manifest is persisted.
    pub blob_manifest: Option<serde_json::Value>,
}

/// Author-supplied summary source (markdown/typst, same format as content).
/// Used verbatim when present; when empty, summary stays empty until the
/// author fills it in (no auto-extraction — tracked on the roadmap as AI summary).
pub(super) struct SummaryInput<'a> {
    pub user_source: Option<&'a str>,
}

/// Where a publish should materialize its source files.
///
/// - `Standalone { at_uri }` is the default: writes to
///   `{blob_cache}/{uri_to_node_id(at_uri)}/` with content/summary filenames
///   derived from the content format (`content.{ext}`, `summary.{ext}`).
/// - `SeriesChapter { series_repo_uri, chapter_path }` writes into the
///   series's shared blob_cache dir using the chapter's path as the entry
///   filename (e.g. `chapters/01-intro.typ`). Summary is suppressed — series
///   chapters don't have per-chapter summaries in the bundle; chapter summary
///   text still lives on `article_localizations.summary` for DB queries.
pub(super) enum PublishTarget<'a> {
    Standalone { at_uri: &'a str },
    SeriesChapter { series_repo_uri: &'a str, chapter_path: &'a str },
}

/// PDS-blob publish path — the one canonical publish path. Uploads
/// `content.{ext}` and `summary.{ext}` as PDS blobs under the author's DID,
/// materializes them into the blob cache for rendering, and returns a
/// `BlobManifest` the caller stores on the `article_localizations` row
/// and mirrors into the `at.nightbo.work` record's `content` union.
///
/// No pijul, no knot, no scratch. Updates are new blobs replacing the old
/// manifest — no change history. Fork (future) will promote a blob article
/// to pijul by initializing a repo with the current manifest files as the
/// initial change.
pub(super) async fn publish_article_blob(
    state: &AppState,
    at_uri: &str,
    did: &str,
    token: &str,
    content: &str,
    format: ContentFormat,
    description: SummaryInput<'_>,
) -> Result<PublishResult, AppError> {
    publish_article_blob_to(
        state, did, token, content, format, description,
        PublishTarget::Standalone { at_uri },
    ).await
}

/// Generalized publish: same as [`publish_article_blob`] but caller picks the
/// destination. Series chapters use `PublishTarget::SeriesChapter` so their
/// source file lands inside the series's shared blob_cache dir under the
/// chapter's path (e.g. `chapters/01-intro.typ`), which is what the typst
/// whole-document compile needs to resolve cross-chapter `@refs`.
pub(super) async fn publish_article_blob_to(
    state: &AppState,
    did: &str,
    token: &str,
    content: &str,
    format: ContentFormat,
    description: SummaryInput<'_>,
    target: PublishTarget<'_>,
) -> Result<PublishResult, AppError> {
    let fmt = format.as_str();
    let src_ext = fx_renderer::format_extension(fmt);

    // Resolve cache dir + the entry filename we write the source to.
    let (cache_path, content_filename, is_series_chapter, stable_uri) = match target {
        PublishTarget::Standalone { at_uri } => {
            let node_id = uri_to_node_id(at_uri);
            (state.blob_cache_path.join(&node_id),
             format!("content.{src_ext}"),
             false,
             at_uri.to_string())
        }
        PublishTarget::SeriesChapter { series_repo_uri, chapter_path } => {
            let node_id = uri_to_node_id(series_repo_uri);
            (state.blob_cache_path.join(&node_id),
             chapter_path.to_string(),
             true,
             series_repo_uri.to_string())
        }
    };
    let summary_filename = format!("summary.{src_ext}");

    // Upload content blob (or synthesize locally for did:local:* users).
    let content_mime = mime_for_source_ext(src_ext);
    let content_blob = crate::auth::upload_or_local_blob(
        state,
        token,
        did,
        content.as_bytes().to_vec(),
        content_mime,
    ).await;

    let summary_source = description.user_source.unwrap_or("").to_string();
    // Series chapters don't carry a separate summary file in the bundle —
    // chapter summary text still lives on article_localizations.summary for
    // DB queries, and lead-block rendering uses the text verbatim.
    let upload_summary_blob = !summary_source.is_empty() && !is_series_chapter;
    let summary_blob = if upload_summary_blob {
        Some(crate::auth::upload_or_local_blob(
            state,
            token,
            did,
            summary_source.clone().into_bytes(),
            content_mime,
        ).await)
    } else {
        None
    };

    // Materialize into blob_cache so the renderer can find source files
    // under the same layout as a pijul working tree (expected by
    // `resolve_location`'s blob branch). For series chapters the entry file
    // may live under a subdir (chapters/01-intro.typ), so create the parent.
    tokio::fs::create_dir_all(&cache_path).await?;
    let entry_abspath = cache_path.join(&content_filename);
    if let Some(parent) = entry_abspath.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&entry_abspath, content).await?;
    if upload_summary_blob {
        tokio::fs::write(cache_path.join(&summary_filename), &summary_source).await?;
    }

    // Initial HTML render for list views + reading page. For standalone
    // articles the cached HTML lives next to the source at
    // `content.html`. For series chapters each chapter gets its own cache
    // file under `cache/{chapter_stem}.html` so cached renders don't collide
    // with each other.
    if format != ContentFormat::Html {
        // Render relative to the entry file's directory so typst/markdown
        // resolve sibling assets. For series chapters the series root is
        // what we want so cross-chapter @refs resolve — pass `cache_path`.
        let render_root = if is_series_chapter {
            cache_path.clone()
        } else {
            cache_path.clone()
        };
        match render_content(fmt, content, &render_root) {
            Ok(rendered) => {
                let cached_html_path = if is_series_chapter {
                    let anchor_stem = std::path::Path::new(&content_filename)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(String::from)
                        .unwrap_or_else(|| "chapter".to_string());
                    cache_path.join("cache").join(format!("{anchor_stem}.html"))
                } else {
                    cache_path.join("content.html")
                };
                if let Some(parent) = cached_html_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let _ = tokio::fs::write(&cached_html_path, &rendered).await;
            }
            Err(e) => tracing::warn!("initial render (blob path) skipped: {e:?}"),
        }
    }

    let summary_html = crate::summary::render_summary_inline(fmt, &summary_source, &cache_path)
        .unwrap_or_else(|e| {
            tracing::warn!("description render failed: {e}");
            String::new()
        });

    // Assemble manifest: entry + files[].
    let mut manifest_files = serde_json::json!([
        {
            "path": content_filename,
            "cid": blob_cid(&content_blob),
            "size": content.len() as u64,
            "mime": content_mime,
        }
    ]).as_array().cloned().unwrap_or_default();
    if let Some(ref sb) = summary_blob {
        manifest_files.push(serde_json::json!({
            "path": summary_filename,
            "cid": blob_cid(sb),
            "size": summary_source.len() as u64,
            "mime": content_mime,
        }));
    }
    let manifest = serde_json::json!({
        "entry": content_filename,
        "files": manifest_files,
        "pds_url": state.pds_url,
    });

    // Sync cover from the source's native metadata to the DB cache. Markdown
    // reads frontmatter, HTML parses <meta>, typst compiles + introspects.
    // Only for standalone articles — series chapters inherit the series
    // cover (set on the series record, not per-chapter).
    if !is_series_chapter {
        let at_uri_for_cover = stable_uri.clone();
        let node_id_for_cover = uri_to_node_id(&at_uri_for_cover);
        let cover_from_source: Option<String> = match fmt {
            "markdown" => fx_core::meta::split_frontmatter(content).0.cover,
            "html"     => super::covers::html_cover_from_meta(content),
            "typst"    => {
                let repo = cache_path.clone();
                tokio::task::spawn_blocking(move || {
                    let config = fx_renderer::fx_render_config();
                    fx_renderer::extract_typst_article_cover(&repo, &config)
                }).await.ok().flatten()
            }
            _ => None,
        };
        if let Some(rel) = cover_from_source {
            let cover_url = format!("/api/covers/a-{}", node_id_for_cover);
            let _ = sqlx::query(
                "UPDATE articles a SET cover_url = $1, cover_file = $2 \
                 FROM article_localizations l \
                 WHERE l.at_uri = $3 AND a.repo_uri = l.repo_uri AND a.source_path = l.source_path",
            )
            .bind(&cover_url).bind(&rel).bind(&at_uri_for_cover)
            .execute(&state.pool).await;
        }
    }

    let _ = stable_uri;
    Ok(PublishResult {
        repo_path: cache_path,
        summary_source,
        summary_html,
        blob_manifest: Some(manifest),
    })
}

fn mime_for_source_ext(ext: &str) -> &'static str {
    match ext {
        "md"   => "text/markdown",
        "typ"  => "text/x-typst",
        "html" => "text/html",
        _      => "text/plain",
    }
}

/// Extract the CID string from a PDS `$type: blob` ref JSON value — the
/// shape returned by `com.atproto.repo.uploadBlob`.
fn blob_cid(blob: &serde_json::Value) -> String {
    blob.get("ref")
        .and_then(|r| r.get("$link"))
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Save category-specific metadata for an article.
async fn save_category_metadata(state: &AppState, at_uri: &str, input: &CreateArticle) {
    match &input.metadata {
        Some(CategoryMetadata::Paper(paper)) => {
            let _ = article_service::upsert_paper_metadata(&state.pool, at_uri, paper).await;
        }
        Some(CategoryMetadata::Experience(exp)) => {
            let _ = article_service::upsert_experience_metadata(&state.pool, at_uri, exp).await;
        }
        Some(CategoryMetadata::Review { .. }) | Some(CategoryMetadata::Note { .. }) => {
            // Review / Note metadata (book_id, edition_id, chapter_id, ...) is
            // stored directly on the articles table via CreateArticle INSERT;
            // no auxiliary table to upsert.
        }
        None => {}
    }
}

/// Build the PDS-side `at.nightbo.work` record body. `content` is the
/// `#content` sub-object (`{entry, contentFormat, files}`), rebuilt from a
/// blob manifest so each file is a proper `$type: blob` ref that external
/// AppViews can fetch via `com.atproto.sync.getBlob`.
pub(super) fn build_article_record(
    input: &CreateArticle,
    manifest: &serde_json::Value,
) -> serde_json::Value {
    let files: Vec<serde_json::Value> = manifest
        .get("files")
        .and_then(|f| f.as_array())
        .map(|arr| arr.iter().map(|f| {
            let cid = f.get("cid").and_then(|c| c.as_str()).unwrap_or_default();
            let mime = f.get("mime").and_then(|m| m.as_str()).unwrap_or("text/plain");
            let size = f.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
            serde_json::json!({
                "path": f.get("path").cloned().unwrap_or(serde_json::Value::Null),
                "blob": {
                    "$type": "blob",
                    "ref": { "$link": cid },
                    "mimeType": mime,
                    "size": size,
                },
                "mime": mime,
            })
        }).collect())
        .unwrap_or_default();
    serde_json::json!({
        "$type": fx_atproto::lexicon::WORK,
        "title": input.title,
        "description": input.summary.as_deref().unwrap_or(""),
        "tags": input.tags,
        "license": input.license.as_deref().unwrap_or(""),
        "content": {
            "entry": manifest.get("entry").cloned().unwrap_or(serde_json::Value::Null),
            "contentFormat": input.content_format.as_str(),
            "files": files,
        },
        "createdAt": now_rfc3339(),
    })
}

/// Set the `cover` blob on a series's PDS record. Fetches the current
/// record, replaces `cover` with the provided blob ref, and writes it back
/// via `putRecord`. Best-effort: failures are logged but do not block.
pub(super) async fn set_cover_on_series_record(
    state: &AppState,
    token: &str,
    owner_did: &str,
    series_id: &str,
    cover_blob: &serde_json::Value,
) {
    let Some(pds) = crate::auth::pds_session(&state.pool, token).await else {
        tracing::warn!("no PDS session; skipping series cover record write for {series_id}");
        return;
    };
    let mut record = match state.at_client.get_record(
        &pds.pds_url, &pds.access_jwt, owner_did,
        fx_atproto::lexicon::WORK, series_id,
    ).await {
        Ok(r) => r.get("value").cloned().unwrap_or_else(|| serde_json::json!({
            "$type": fx_atproto::lexicon::WORK,
            "seriesId": series_id,
            "title": "",
            "chapters": [],
            "createdAt": now_rfc3339(),
        })),
        Err(e) => {
            tracing::warn!("series record {series_id} fetch failed (will create fresh): {e}");
            serde_json::json!({
                "$type": fx_atproto::lexicon::WORK,
                "seriesId": series_id,
                "title": "",
                "chapters": [],
                "createdAt": now_rfc3339(),
            })
        }
    };
    record["cover"] = cover_blob.clone();
    if let Err(e) = state.at_client.put_record(
        &pds.pds_url, &pds.access_jwt,
        &fx_atproto::client::PutRecordInput {
            repo: pds.did.clone(),
            collection: fx_atproto::lexicon::WORK.to_string(),
            rkey: series_id.to_string(),
            record,
        },
    ).await {
        crate::auth::log_pds_error("set cover on series record", e);
    }
}

/// Merge a chapter into a series's PDS record.
///
/// Under the unified-series lexicon, a series is ONE big record that carries
/// every chapter's metadata (`chapters[]`) and every source/asset file
/// (`files[]`). There is no per-chapter PDS record. On publish we:
///   1. Fetch the current series record (or seed a skeleton if absent).
///   2. Upsert the chapter entry (`{title, entry, anchor, orderIndex}`) into
///      `chapters[]` — replacing any existing item whose `entry` matches so
///      re-publish is idempotent.
///   3. Merge `chapter_files` into `files[]`, deduped by `path`. Replaces
///      existing entries when paths collide (blob CIDs change on re-publish).
///   4. putRecord the updated value back to the user's PDS.
///
/// Best-effort: without a PDS session we log + skip. DB side-effects happen
/// regardless; the PDS lags until the author re-publishes under a real
/// session.
pub(super) async fn merge_chapter_into_series_record(
    state: &AppState,
    token: &str,
    owner_did: &str,
    series_id: &str,
    title: &str,
    entry: &str,
    anchor: &str,
    order_index: i64,
    chapter_files: &[serde_json::Value],
) {
    let Some(pds) = crate::auth::pds_session(&state.pool, token).await else {
        tracing::debug!("no PDS session; skipping series merge for {series_id}");
        return;
    };

    // Load existing record (series records are keyed by series id).
    let mut record = match state.at_client.get_record(
        &pds.pds_url, &pds.access_jwt, owner_did,
        fx_atproto::lexicon::WORK, series_id,
    ).await {
        Ok(r) => r.get("value").cloned().unwrap_or_else(|| seed_series_record(series_id)),
        Err(e) => {
            tracing::warn!("series record {series_id} fetch failed (will create fresh): {e}");
            seed_series_record(series_id)
        }
    };

    // 1. Upsert chapter entry by `entry` path — stable identity across
    //    re-publishes.
    let new_chapter = serde_json::json!({
        "title":      title,
        "entry":      entry,
        "anchor":     anchor,
        "orderIndex": order_index,
    });
    let chapters = record
        .get_mut("chapters")
        .and_then(|c| c.as_array_mut());
    match chapters {
        Some(arr) => {
            if let Some(pos) = arr.iter().position(|c| {
                c.get("entry").and_then(|e| e.as_str()) == Some(entry)
            }) {
                arr[pos] = new_chapter;
            } else {
                arr.push(new_chapter);
            }
        }
        None => {
            record["chapters"] = serde_json::json!([new_chapter]);
        }
    }

    // 2. Merge chapter files into `files[]`, deduped by path.
    let files = record
        .get_mut("files")
        .and_then(|c| c.as_array_mut());
    match files {
        Some(arr) => {
            for f in chapter_files {
                let Some(p) = f.get("path").and_then(|p| p.as_str()) else { continue };
                if let Some(pos) = arr.iter().position(|ex| {
                    ex.get("path").and_then(|x| x.as_str()) == Some(p)
                }) {
                    arr[pos] = f.clone();
                } else {
                    arr.push(f.clone());
                }
            }
        }
        None => {
            record["files"] = serde_json::Value::Array(chapter_files.to_vec());
        }
    }

    if let Err(e) = state.at_client.put_record(
        &pds.pds_url, &pds.access_jwt,
        &fx_atproto::client::PutRecordInput {
            repo: pds.did.clone(),
            collection: fx_atproto::lexicon::WORK.to_string(),
            rkey: series_id.to_string(),
            record,
        },
    ).await {
        crate::auth::log_pds_error("merge chapter into series record", e);
    }
}

fn seed_series_record(series_id: &str) -> serde_json::Value {
    serde_json::json!({
        "$type":       fx_atproto::lexicon::WORK,
        "seriesId":    series_id,
        "title":       "",
        "contentFormat": "typst",
        "chapters":    [],
        "files":       [],
        "createdAt":   now_rfc3339(),
    })
}

/// Build the per-chapter `files[]` entries (shape expected by the series
/// record lexicon: `{path, blob, mime}`) from a chapter's blob manifest.
/// Used by `merge_chapter_into_series_record` callers.
pub(super) fn chapter_file_refs_from_manifest(manifest: &serde_json::Value) -> Vec<serde_json::Value> {
    manifest
        .get("files")
        .and_then(|f| f.as_array())
        .map(|arr| arr.iter().map(|f| {
            let cid = f.get("cid").and_then(|c| c.as_str()).unwrap_or_default();
            let mime = f.get("mime").and_then(|m| m.as_str()).unwrap_or("text/plain");
            let size = f.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
            serde_json::json!({
                "path": f.get("path").cloned().unwrap_or(serde_json::Value::Null),
                "blob": {
                    "$type": "blob",
                    "ref": { "$link": cid },
                    "mimeType": mime,
                    "size": size,
                },
                "mime": mime,
            })
        }).collect())
        .unwrap_or_default()
}

pub async fn create_article(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateArticle>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    validate_create_article(&input)?;

    // Verify series ownership if series_id is specified.
    if let Some(ref sid) = input.series_id {
        let owner = series_service::get_series_owner(&state.pool, sid).await?;
        require_owner(Some(&owner), &user.did)?;
    }

    if input.series_id.is_some() {
        create_series_chapter_handler(&state, &user, &input).await
    } else {
        create_standalone_article_handler(&state, &user, &input).await
    }
}

/// Standalone-article publish: mints its own at_uri, writes its own PDS
/// record, and returns the Article view.
async fn create_standalone_article_handler(
    state: &AppState,
    user: &crate::auth::AuthUser,
    input: &CreateArticle,
) -> ApiResult<(StatusCode, Json<Article>)> {
    let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::WORK, tid());

    let publish = publish_article_blob(
        state, &at_uri, &user.did, &user.token, &input.content, input.content_format,
        SummaryInput { user_source: input.summary.as_deref() },
    ).await?;

    let hash = content_hash(&input.content);

    let translation_group = if let Some(ref source_uri) = input.translation_of {
        #[allow(deprecated)]
        Some(article_service::resolve_translation_group(&state.pool, source_uri).await?)
    } else {
        None
    };

    let article = article_service::create_article(
        &state.pool, &user.did, &at_uri, input, &hash, translation_group,
        default_visibility(user.phone_verified), ContentKind::Article, None,
        &publish.summary_source, &publish.summary_html,
        publish.blob_manifest.clone(),
    ).await?;

    if !input.restricted.unwrap_or(false) {
        if let Some(ref manifest) = publish.blob_manifest {
            let record = build_article_record(input, manifest);
            let rkey = at_uri.rsplit('/').next().map(str::to_string);
            pds_create_record(state, &user.token, fx_atproto::lexicon::WORK, record, rkey, "create article").await;
        }
    }

    let _ = article_service::auto_bookmark(&state.pool, &user.did, &at_uri).await;
    let _ = collaboration_service::register_article_owner(&state.pool, &at_uri, &user.did).await;
    save_category_metadata(state, &at_uri, input).await;

    let authorship_record = serde_json::json!({
        "$type": fx_atproto::lexicon::AUTHORSHIP,
        "article": at_uri,
        "createdAt": now_rfc3339(),
    });
    let authorship_uri = pds_create_record(
        state, &user.token, fx_atproto::lexicon::AUTHORSHIP, authorship_record, None, "creator authorship",
    ).await;
    let _ = authorship_service::add_author(&state.pool, &at_uri, &user.did, &user.did, Some(0)).await;
    if let Some(ref uri) = authorship_uri {
        let _ = authorship_service::verify_authorship(&state.pool, &at_uri, &user.did, Some(uri)).await;
    }

    for (i, author_did) in input.authors.iter().enumerate() {
        if author_did != &user.did {
            let _ = authorship_service::add_author(
                &state.pool, &at_uri, author_did, &user.did, Some((i + 1) as i16),
            ).await;
        }
    }

    Ok((StatusCode::CREATED, Json(article)))
}

/// Series-chapter publish: no per-chapter at_uri and no per-chapter PDS
/// record. Source file + manifest are written into the SERIES's shared
/// blob_cache dir, a chapter row with `at_uri = NULL` is inserted, and the
/// series's `at.nightbo.work` record is merged in-place with the new
/// chapter entry + file refs.
async fn create_series_chapter_handler(
    state: &AppState,
    user: &crate::auth::AuthUser,
    input: &CreateArticle,
) -> ApiResult<(StatusCode, Json<Article>)> {
    let series_id = input.series_id.clone()
        .ok_or_else(|| AppError(fx_core::Error::BadRequest("series_id required".into())))?;
    let series_repo_uri = series_service::series_repo_uri(&state.pool, &series_id).await?;

    // Derive a stable chapter path (source_path within the series bundle).
    // Using a fresh tid keeps default paths unique even when multiple
    // chapters are created in the same second. Authors can override the
    // path via the CLI/batch-publish path (future).
    let chapter_tid = tid();
    let src_ext = fx_renderer::format_extension(input.content_format.as_str());
    let source_path = fx_core::meta::default_chapter_path(&chapter_tid, src_ext);

    let publish = publish_article_blob_to(
        state, &user.did, &user.token, &input.content, input.content_format,
        SummaryInput { user_source: input.summary.as_deref() },
        PublishTarget::SeriesChapter {
            series_repo_uri: &series_repo_uri,
            chapter_path: &source_path,
        },
    ).await?;

    let hash = content_hash(&input.content);
    let anchor = chapter_tid.clone();

    let article = article_service::create_series_chapter(
        &state.pool, &user.did, &series_repo_uri, &source_path,
        input, &hash,
        default_visibility(user.phone_verified),
        &publish.summary_source, &publish.summary_html,
        publish.blob_manifest.clone(),
    ).await?;

    // Order-index is assigned by add_series_chapter and returned so we emit
    // the same value into the series record's `chapters[]`.
    let order_index = series_service::add_series_chapter(
        &state.pool, &series_id, &series_repo_uri, &source_path, Some(&anchor),
    ).await?;

    // Merge this chapter into the series's PDS record (chapters[] + files[]).
    if !input.restricted.unwrap_or(false) {
        if let Some(ref manifest) = publish.blob_manifest {
            let file_refs = chapter_file_refs_from_manifest(manifest);
            merge_chapter_into_series_record(
                state, &user.token, &user.did, &series_id,
                &input.title, &source_path, &anchor,
                order_index as i64, &file_refs,
            ).await;
        }
    }

    // Chapter-specific metadata (book_id / course_id / note fields) is
    // already persisted on the `articles` row by create_series_chapter via
    // CreateArticle's target_* accessors. Paper/Experience metadata do not
    // apply to series chapters (they describe whole publications/events, not
    // parts of a larger series work), so we skip save_category_metadata
    // entirely here.
    //
    // No per-chapter authorship record either — series-level authorship
    // attributes the whole work. No auto_bookmark because chapters aren't
    // individually bookmarkable in the new model (you bookmark the series).

    Ok((StatusCode::CREATED, Json(article)))
}

/// Create an article with all resources in a single multipart request.
/// Fields:
///   - `metadata` (text/json): CreateArticle JSON (same as POST /articles body)
///   - `resources` (file, repeatable): resource files with relative paths as filenames
pub async fn create_article_multipart(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<Article>)> {
    let mut metadata_json: Option<String> = None;
    let mut resources: Vec<(String, Vec<u8>)> = Vec::new(); // (relative_path, data)

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError(fx_core::Error::BadRequest(format!("Multipart error: {e}")))
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "metadata" => {
                metadata_json = Some(field.text().await.map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?);
            }
            "resources" => {
                let filename = field.file_name().unwrap_or("unknown").to_string();
                let data = field.bytes().await.map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?.to_vec();
                if data.len() > MAX_IMAGE_SIZE {
                    return Err(AppError(fx_core::Error::BadRequest(format!("Resource too large: {filename}"))));
                }
                resources.push((filename, data));
            }
            _ => {}
        }
    }

    let input: CreateArticle = serde_json::from_str(
        &metadata_json.ok_or(AppError(fx_core::Error::BadRequest("Missing metadata field".into())))?
    ).map_err(|e| AppError(fx_core::Error::BadRequest(format!("Invalid metadata: {e}"))))?;

    validate_create_article(&input)?;

    if let Some(ref sid) = input.series_id {
        let owner = series_service::get_series_owner(&state.pool, sid).await?;
        require_owner(Some(&owner), &user.did)?;
    }

    if input.series_id.is_some() {
        create_series_chapter_multipart(&state, &user, &input, &resources).await
    } else {
        create_standalone_multipart(&state, &user, &input, &resources).await
    }
}

async fn create_standalone_multipart(
    state: &AppState,
    user: &crate::auth::AuthUser,
    input: &CreateArticle,
    resources: &[(String, Vec<u8>)],
) -> ApiResult<(StatusCode, Json<Article>)> {
    let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::WORK, tid());

    // Upload extra resources as PDS blobs and stage them alongside the entry
    // file.
    let mut extra_blobs: Vec<(String, serde_json::Value, usize, &'static str)> = Vec::new();
    let node_id = uri_to_node_id(&at_uri);
    let stage_path = state.blob_cache_path.join(&node_id);
    tokio::fs::create_dir_all(&stage_path).await?;
    for (rel_path, data) in resources {
        let safe_path: String = rel_path.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' { c } else { '_' })
            .collect();
        let safe_path = safe_path.trim_start_matches('.').trim_start_matches('/').to_string();
        if safe_path.is_empty() || safe_path.contains("..") {
            continue;
        }
        let dest = stage_path.join(&safe_path);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&dest, data).await?;

        let mime = mime_for_source_ext(
            std::path::Path::new(&safe_path).extension().and_then(|e| e.to_str()).unwrap_or(""),
        );
        if let Some(blob) = crate::auth::pds_upload_blob(state, &user.token, data.clone(), mime).await {
            extra_blobs.push((safe_path, blob, data.len(), mime));
        }
    }

    let mut publish = publish_article_blob(
        state, &at_uri, &user.did, &user.token, &input.content, input.content_format,
        SummaryInput { user_source: input.summary.as_deref() },
    ).await?;

    if !extra_blobs.is_empty() {
        if let Some(ref mut manifest) = publish.blob_manifest {
            let files = manifest.get_mut("files")
                .and_then(|f| f.as_array_mut())
                .expect("blob manifest files[]");
            for (path, blob, size, mime) in &extra_blobs {
                files.push(serde_json::json!({
                    "path": path,
                    "cid": blob_cid(blob),
                    "size": *size as u64,
                    "mime": *mime,
                }));
            }
        }
    }

    let hash = content_hash(&input.content);

    let translation_group = if let Some(ref source_uri) = input.translation_of {
        #[allow(deprecated)]
        Some(article_service::resolve_translation_group(&state.pool, source_uri).await?)
    } else {
        None
    };

    let article = article_service::create_article(
        &state.pool, &user.did, &at_uri, input, &hash, translation_group,
        default_visibility(user.phone_verified), ContentKind::Article, None,
        &publish.summary_source, &publish.summary_html,
        publish.blob_manifest.clone(),
    ).await?;

    if !input.restricted.unwrap_or(false) {
        if let Some(ref manifest) = publish.blob_manifest {
            let record = build_article_record(input, manifest);
            let rkey = at_uri.rsplit('/').next().map(str::to_string);
            pds_create_record(state, &user.token, fx_atproto::lexicon::WORK, record, rkey, "create article").await;
        }
    }

    let _ = article_service::auto_bookmark(&state.pool, &user.did, &at_uri).await;
    let _ = collaboration_service::register_article_owner(&state.pool, &at_uri, &user.did).await;

    save_category_metadata(state, &at_uri, input).await;

    let authorship_record = serde_json::json!({
        "$type": fx_atproto::lexicon::AUTHORSHIP,
        "article": at_uri,
        "createdAt": now_rfc3339(),
    });
    let authorship_uri = pds_create_record(
        state, &user.token, fx_atproto::lexicon::AUTHORSHIP, authorship_record, None, "creator authorship",
    ).await;
    let _ = authorship_service::add_author(&state.pool, &at_uri, &user.did, &user.did, Some(0)).await;
    if let Some(ref uri) = authorship_uri {
        let _ = authorship_service::verify_authorship(&state.pool, &at_uri, &user.did, Some(uri)).await;
    }

    for (i, author_did) in input.authors.iter().enumerate() {
        if author_did != &user.did {
            let _ = authorship_service::add_author(
                &state.pool, &at_uri, author_did, &user.did, Some((i + 1) as i16),
            ).await;
        }
    }

    Ok((StatusCode::CREATED, Json(article)))
}

async fn create_series_chapter_multipart(
    state: &AppState,
    user: &crate::auth::AuthUser,
    input: &CreateArticle,
    resources: &[(String, Vec<u8>)],
) -> ApiResult<(StatusCode, Json<Article>)> {
    let series_id = input.series_id.clone()
        .ok_or_else(|| AppError(fx_core::Error::BadRequest("series_id required".into())))?;
    let series_repo_uri = series_service::series_repo_uri(&state.pool, &series_id).await?;

    // Stage extra resources into the SERIES blob cache (shared across chapters).
    let mut extra_blobs: Vec<(String, serde_json::Value, usize, &'static str)> = Vec::new();
    let series_node_id = uri_to_node_id(&series_repo_uri);
    let stage_path = state.blob_cache_path.join(&series_node_id);
    tokio::fs::create_dir_all(&stage_path).await?;
    for (rel_path, data) in resources {
        let safe_path: String = rel_path.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' { c } else { '_' })
            .collect();
        let safe_path = safe_path.trim_start_matches('.').trim_start_matches('/').to_string();
        if safe_path.is_empty() || safe_path.contains("..") {
            continue;
        }
        let dest = stage_path.join(&safe_path);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&dest, data).await?;

        let mime = mime_for_source_ext(
            std::path::Path::new(&safe_path).extension().and_then(|e| e.to_str()).unwrap_or(""),
        );
        if let Some(blob) = crate::auth::pds_upload_blob(state, &user.token, data.clone(), mime).await {
            extra_blobs.push((safe_path, blob, data.len(), mime));
        }
    }

    let chapter_tid = tid();
    let src_ext = fx_renderer::format_extension(input.content_format.as_str());
    let source_path = fx_core::meta::default_chapter_path(&chapter_tid, src_ext);
    let anchor = chapter_tid.clone();

    let mut publish = publish_article_blob_to(
        state, &user.did, &user.token, &input.content, input.content_format,
        SummaryInput { user_source: input.summary.as_deref() },
        PublishTarget::SeriesChapter {
            series_repo_uri: &series_repo_uri,
            chapter_path: &source_path,
        },
    ).await?;

    // Fold extra resources into the chapter's manifest — but tag them with
    // paths relative to the series root (that's how the series bundle sees
    // them). `extra_blobs` was keyed by user-supplied paths already relative
    // to the series root, so no rewrite needed.
    if !extra_blobs.is_empty() {
        if let Some(ref mut manifest) = publish.blob_manifest {
            let files = manifest.get_mut("files")
                .and_then(|f| f.as_array_mut())
                .expect("blob manifest files[]");
            for (path, blob, size, mime) in &extra_blobs {
                files.push(serde_json::json!({
                    "path": path,
                    "cid": blob_cid(blob),
                    "size": *size as u64,
                    "mime": *mime,
                }));
            }
        }
    }

    let hash = content_hash(&input.content);

    let article = article_service::create_series_chapter(
        &state.pool, &user.did, &series_repo_uri, &source_path,
        input, &hash,
        default_visibility(user.phone_verified),
        &publish.summary_source, &publish.summary_html,
        publish.blob_manifest.clone(),
    ).await?;

    let order_index = series_service::add_series_chapter(
        &state.pool, &series_id, &series_repo_uri, &source_path, Some(&anchor),
    ).await?;

    if !input.restricted.unwrap_or(false) {
        if let Some(ref manifest) = publish.blob_manifest {
            let file_refs = chapter_file_refs_from_manifest(manifest);
            merge_chapter_into_series_record(
                state, &user.token, &user.did, &series_id,
                &input.title, &source_path, &anchor,
                order_index as i64, &file_refs,
            ).await;
        }
    }

    Ok((StatusCode::CREATED, Json(article)))
}

// --- Full article page data (single request) ---

#[derive(serde::Serialize)]
pub struct ArticleFullResponse {
    article: Article,
    content: ArticleContent,
    prereqs: Vec<ArticlePrereqRow>,
    /// Tag ids this article mentions without teaching (content_related).
    related: Vec<String>,
    forks: Vec<ForkWithTitle>,
    fork_source: Option<fx_core::services::article_service::ForkSourceInfo>,
    votes: ArticleVoteSummary,
    series_context: Vec<fx_core::services::series_service::SeriesContextItem>,
    translations: Vec<Article>,
    #[serde(skip_serializing_if = "Option::is_none")]
    paper: Option<PaperMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    experience: Option<ExperienceMetadata>,
    my_vote: i32,
    is_bookmarked: bool,
    learned: bool,
    access_denied: bool,
}

#[derive(serde::Serialize)]
struct ArticleVoteSummary {
    score: i64,
    upvotes: i64,
    downvotes: i64,
}

pub async fn get_article_full(
    State(state): State<AppState>,
    crate::auth::MaybeAuth(user): crate::auth::MaybeAuth,
    Query(q): Query<PrereqsQuery>,
) -> ApiResult<Json<ArticleFullResponse>> {
    use fx_core::services::{vote_service, bookmark_service, series_service, learned_service};
    let uri = q.uri.clone();
    let locale = q.locale.as_deref().unwrap_or("en").to_string();

    let mode = state.instance_mode;
    let (article, prereqs, related, forks, vote_summary, series_ctx, translations) = tokio::try_join!(
        article_service::get_article(&state.pool, mode, &uri),
        article_service::get_article_prereqs(&state.pool, &uri, &locale),
        article_service::get_article_related(&state.pool, &uri),
        article_service::get_article_forks(&state.pool, &uri),
        vote_service::get_vote_summary(&state.pool, &uri),
        series_service::get_series_context(&state.pool, &uri),
        article_service::get_translations(&state.pool, mode, &uri),
    ).map_err(AppError)?;

    // Access control check
    let has_access = article_service::check_content_access(
        &state.pool, &uri, user.as_ref().map(|u| u.did.as_str()),
    ).await?;

    let content = if has_access {
        resolve_article_content(&state, &uri, article.content_format.as_str()).await?
    } else {
        ArticleContent { source: String::new(), html: String::new() }
    };

    let (my_vote, is_bookmarked, learned) = if let Some(ref u) = user {
        let (mv, bk, lr) = tokio::join!(
            vote_service::get_my_vote(&state.pool, &uri, &u.did),
            bookmark_service::list_bookmarks(&state.pool, &u.did),
            learned_service::is_learned(&state.pool, &u.did, &uri),
        );
        (
            mv.unwrap_or(0),
            bk.map(|bks| bks.iter().any(|b| b.article_uri == uri)).unwrap_or(false),
            lr.unwrap_or(false),
        )
    } else {
        (0, false, false)
    };

    let fork_source = article_service::get_fork_source(&state.pool, &uri).await.unwrap_or(None);

    let paper = if article.category == "paper" {
        article_service::get_paper_metadata(&state.pool, &uri).await.unwrap_or(None)
    } else {
        None
    };
    let experience = if article.category == "experience" {
        article_service::get_experience_metadata(&state.pool, &uri).await.unwrap_or(None)
    } else {
        None
    };

    Ok(Json(ArticleFullResponse {
        article,
        content,
        prereqs,
        related,
        forks,
        fork_source,
        votes: ArticleVoteSummary {
            score: vote_summary.score,
            upvotes: vote_summary.upvotes,
            downvotes: vote_summary.downvotes,
        },
        series_context: series_ctx,
        translations,
        paper,
        experience,
        my_vote,
        is_bookmarked,
        learned,
        access_denied: !has_access,
    }))
}

// --- Bulk article metadata ---

#[derive(serde::Deserialize)]
pub struct BulkLimitQuery {
    pub limit: Option<i64>,
}

pub async fn get_all_article_teaches(
    State(state): State<AppState>,
    Query(q): Query<BulkLimitQuery>,
) -> ApiResult<Json<Vec<article_service::ContentTeachRow>>> {
    let limit = q.limit.unwrap_or(10_000).clamp(1, 50_000);
    let rows = article_service::get_all_article_teaches(&state.pool, limit).await?;
    Ok(Json(rows))
}

pub async fn get_all_article_prereqs(
    State(state): State<AppState>,
    Query(q): Query<BulkLimitQuery>,
) -> ApiResult<Json<Vec<article_service::ContentPrereqBulkRow>>> {
    let limit = q.limit.unwrap_or(10_000).clamp(1, 50_000);
    let rows = article_service::get_all_article_prereqs(&state.pool, limit).await?;
    Ok(Json(rows))
}

#[derive(serde::Deserialize)]
pub struct TagArticlesQuery {
    pub tag_id: String,
    pub limit: Option<i64>,
}

pub async fn get_articles_by_tag(
    State(state): State<AppState>,
    Query(q): Query<TagArticlesQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let Some(tag_id) = fx_core::services::tag_service::lookup_tag_id(&state.pool, &q.tag_id).await? else {
        return Ok(Json(vec![]));
    };
    let articles = article_service::get_articles_by_tag(&state.pool, state.instance_mode, &tag_id, limit).await?;
    Ok(Json(articles))
}

pub async fn get_articles_related_by_tag(
    State(state): State<AppState>,
    Query(q): Query<TagArticlesQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let Some(tag_id) = fx_core::services::tag_service::lookup_tag_id(&state.pool, &q.tag_id).await? else {
        return Ok(Json(vec![]));
    };
    let articles = article_service::get_articles_related_by_tag(&state.pool, state.instance_mode, &tag_id, limit).await?;
    Ok(Json(articles))
}

#[derive(serde::Deserialize)]
pub struct DidArticlesQuery {
    pub did: String,
    pub limit: Option<i64>,
}

pub async fn get_articles_by_did(
    State(state): State<AppState>,
    Query(q): Query<DidArticlesQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let articles = article_service::get_articles_by_did(&state.pool, state.instance_mode, &q.did, limit).await?;
    Ok(Json(articles))
}

pub async fn get_translations(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let articles = article_service::get_translations(&state.pool, state.instance_mode, &uri).await?;
    Ok(Json(articles))
}

// --- Format conversion ---

#[derive(serde::Deserialize)]
pub struct ConvertInput {
    content: String,
    from: String,
    to: String,
}

#[derive(serde::Serialize)]
pub struct ConvertOutput {
    content: String,
}

pub async fn convert_content(
    Json(input): Json<ConvertInput>,
) -> ApiResult<Json<ConvertOutput>> {
    if let Err(e) = fx_core::validation::validate_content_format(&input.from) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }
    if let Err(e) = fx_core::validation::validate_content_format(&input.to) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }
    let converted = fx_renderer::convert_format(&input.content, &input.from, &input.to)
        .map_err(|e| AppError(fx_core::Error::Render(e.to_string())))?;
    Ok(Json(ConvertOutput { content: converted }))
}

// --- Image upload ---

const MAX_IMAGE_SIZE: usize = 50 * 1024 * 1024;
const ALLOWED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "svg", "webp", "pdf", "bib", "csv", "json", "toml", "yaml", "txt"];
const RESERVED_CONTENT_NAMES: &[&str] = &["content.typ", "content.md", "content.html", "content.tex", "meta.json"];

pub async fn upload_image(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    mut multipart: Multipart,
) -> ApiResult<Json<ImageUploadResponse>> {
    let mut article_uri: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError(fx_core::Error::BadRequest(format!("Multipart error: {e}")))
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "article_uri" => {
                article_uri = Some(field.text().await.map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?);
            }
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                file_data = Some(field.bytes().await.map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?.to_vec());
            }
            _ => {}
        }
    }

    let uri = article_uri.ok_or(AppError(fx_core::Error::BadRequest("Missing article_uri".into())))?;
    let original_name = file_name.ok_or(AppError(fx_core::Error::BadRequest("Missing file".into())))?;
    let data = file_data.ok_or(AppError(fx_core::Error::BadRequest("Missing file data".into())))?;

    if data.len() > MAX_IMAGE_SIZE {
        return Err(AppError(fx_core::Error::BadRequest("File too large (max 10MB)".into())));
    }

    let owner = article_service::get_article_owner(&state.pool, &uri).await?;
    require_owner(Some(&owner), &user.did)?;

    let ext = std::path::Path::new(&original_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest(format!("Unsupported file type: {ext}"))));
    }

    // Sanitize filename, preserving '/' for subdirectory structure
    let safe_name: String = original_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' { c } else { '_' })
        .collect();
    let safe_name = safe_name.trim_start_matches('.').trim_start_matches('/').to_string();
    if safe_name.is_empty() || safe_name.contains("..") || RESERVED_CONTENT_NAMES.contains(&safe_name.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest("invalid file name".into())));
    }

    // Upload the resource as a PDS blob and stage it into the article's
    // blob_cache dir so the renderer sees it alongside the entry file.
    let node_id = uri_to_node_id(&uri);
    let stage_path = state.blob_cache_path.join(&node_id);
    let dest = stage_path.join(&safe_name);
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&dest, &data).await?;

    // Invalidate any cached HTML so the next read triggers a fresh render
    // that picks up the new resource.
    let _ = tokio::fs::remove_file(stage_path.join("content.html")).await;

    let mime = mime_for_source_ext(
        std::path::Path::new(&safe_name).extension().and_then(|e| e.to_str()).unwrap_or(""),
    );
    let size = data.len();
    let blob = crate::auth::pds_upload_blob(&state, &user.token, data.clone(), mime).await;

    // Patch the article's manifest + PDS record so the resource shows up in
    // `content.files[]`. Failures are non-fatal — the local filesystem copy
    // still renders correctly; the record just falls out of sync until the
    // next full publish.
    if let Some(blob_ref) = blob {
        if let Err(e) = append_file_to_article_manifest(
            &state, &uri, &safe_name, &blob_ref, size, mime,
        ).await {
            tracing::warn!("append file to manifest failed: {e:?}");
        }
        if let Err(e) = append_file_to_article_record(
            &state, &user.token, &user.did, &uri, &safe_name, &blob_ref, size, mime,
        ).await {
            tracing::warn!("patch article record failed: {e:?}");
        }
    }

    Ok(Json(ImageUploadResponse { filename: safe_name }))
}

/// Patch the article's `content_manifest` JSONB (used by `ensure_blob_materialized`
/// on read paths) to include the newly uploaded resource.
pub(super) async fn append_file_to_article_manifest(
    state: &AppState,
    uri: &str,
    path: &str,
    blob: &serde_json::Value,
    size: usize,
    mime: &str,
) -> Result<(), AppError> {
    let existing: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT content_manifest FROM article_localizations WHERE at_uri = $1",
    )
    .bind(uri)
    .fetch_optional(&state.pool)
    .await?
    .flatten();
    let mut manifest = existing.unwrap_or_else(|| serde_json::json!({
        "entry": "",
        "files": [],
        "pds_url": state.pds_url,
    }));
    let files = manifest.get_mut("files").and_then(|f| f.as_array_mut());
    let entry = serde_json::json!({
        "path": path,
        "cid": blob_cid(blob),
        "size": size as u64,
        "mime": mime,
    });
    match files {
        Some(arr) => {
            // Replace if this path already has an entry; otherwise append.
            if let Some(pos) = arr.iter().position(|f| f.get("path").and_then(|p| p.as_str()) == Some(path)) {
                arr[pos] = entry;
            } else {
                arr.push(entry);
            }
        }
        None => {
            manifest["files"] = serde_json::json!([entry]);
        }
    }
    sqlx::query("UPDATE article_localizations SET content_manifest = $1 WHERE at_uri = $2")
        .bind(&manifest)
        .bind(uri)
        .execute(&state.pool)
        .await?;
    Ok(())
}

/// Re-fetch the article record on the user's PDS, append the new file entry
/// to `content.files[]`, and putRecord it back. Doesn't touch content.entry
/// or any other field.
pub(super) async fn append_file_to_article_record(
    state: &AppState,
    token: &str,
    did: &str,
    article_uri: &str,
    path: &str,
    blob: &serde_json::Value,
    size: usize,
    mime: &str,
) -> Result<(), AppError> {
    let Some(rkey) = article_uri.rsplit('/').next() else {
        return Ok(());
    };
    let Some(pds) = crate::auth::pds_session(&state.pool, token).await else {
        return Ok(());
    };
    let existing = match state.at_client.get_record(
        &pds.pds_url, &pds.access_jwt, did,
        fx_atproto::lexicon::WORK, rkey,
    ).await {
        Ok(r) => r,
        Err(e) => {
            crate::auth::log_pds_error("get article record", e);
            return Ok(());
        }
    };
    let mut record = existing.get("value").cloned().unwrap_or_else(|| serde_json::json!({}));
    let files = record.pointer_mut("/content/files").and_then(|f| f.as_array_mut());
    let entry = serde_json::json!({
        "path": path,
        "blob": blob,
        "mime": mime,
    });
    match files {
        Some(arr) => {
            if let Some(pos) = arr.iter().position(|f| f.get("path").and_then(|p| p.as_str()) == Some(path)) {
                arr[pos] = entry;
            } else {
                arr.push(entry);
            }
        }
        None => {
            // No content at all yet — nothing sensible to patch; skip.
            let _ = size; // silence unused warning in this branch
            return Ok(());
        }
    }
    if let Err(e) = state.at_client.put_record(
        &pds.pds_url, &pds.access_jwt,
        &fx_atproto::client::PutRecordInput {
            repo: pds.did.clone(),
            collection: fx_atproto::lexicon::WORK.to_string(),
            rkey: rkey.to_string(),
            record,
        },
    ).await {
        crate::auth::log_pds_error("put article record", e);
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct ImageUploadResponse {
    pub filename: String,
}

// --- Serve article images ---

#[derive(serde::Deserialize)]
pub struct ImageQuery {
    pub uri: String,
    pub name: String,
}

pub async fn get_image(
    State(state): State<AppState>,
    Query(q): Query<ImageQuery>,
) -> ApiResult<(axum::http::HeaderMap, Vec<u8>)> {
    let node_id = uri_to_node_id(&q.uri);

    // Sanitize: allow subdirectories (e.g. _rendered/hash.png, Figure/img.pdf) but reject ..
    let name = &q.name;
    if name.is_empty() || name.contains("..") || name.starts_with('/') {
        return Err(AppError(fx_core::Error::BadRequest("invalid file name".into())));
    }

    // Serve resources from the article's materialized blob cache. Files the
    // renderer emitted into `_rendered/*.svg` etc. sit next to the entry.
    let path = state.blob_cache_path.join(&node_id);
    if let Some(manifest) = load_blob_manifest(&state.pool, &q.uri).await {
        let did = uri_did(&q.uri).ok_or_else(|| AppError(fx_core::Error::BadRequest("bad uri".into())))?;
        ensure_blob_materialized(&state, did, &manifest, &path).await?;
    }
    let repo_path = path;

    let path = repo_path.join(name);
    let data = tokio::fs::read(&path).await.map_err(|_| {
        AppError(fx_core::Error::NotFound { entity: "image", id: name.to_string() })
    })?;

    let content_type = match std::path::Path::new(name).extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream",
    };

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("content-type", content_type.parse().expect("valid content-type"));
    headers.insert("cache-control", "public, max-age=86400".parse().expect("valid cache-control"));
    Ok((headers, data))
}

// --- Update article ---

#[derive(serde::Deserialize)]
pub struct UpdateArticleInput {
    pub uri: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub commit_message: Option<String>,
    /// Category-specific metadata to update.
    pub metadata: Option<CategoryMetadata>,
    /// When false, only saves content to working copy without creating a pijul change.
    #[serde(default = "default_true")]
    pub record: bool,
}

fn default_true() -> bool { true }

pub async fn update_article(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UpdateArticleInput>,
) -> ApiResult<Json<Article>> {
    let mut errors = Vec::new();
    if let Some(ref title) = input.title {
        if let Err(e) = fx_core::validation::validate_title(title) {
            errors.push(e);
        }
    }
    if let Some(ref content) = input.content {
        if let Err(e) = fx_core::validation::validate_article_content(content) {
            errors.push(e);
        }
    }
    if !errors.is_empty() {
        return Err(AppError(fx_core::Error::Validation(errors)));
    }

    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;

    if let Some(ref title) = input.title {
        article_service::update_article_title(&state.pool, &input.uri, title).await?;
    }

    let format = article_service::get_content_format(&state.pool, &input.uri).await?;
    let node_id = uri_to_node_id(&input.uri);
    let repo_path = state.blob_cache_path.join(&node_id);

    let summary_html = if let Some(ref summary) = input.summary {
        let html = crate::summary::render_summary_inline(format.as_str(), summary, &repo_path)
            .unwrap_or_default();
        Some(html)
    } else { None };

    // Content updates re-publish the entry blob. We rebuild the manifest so
    // the blob-cache copy matches the PDS record. Non-entry resources stay
    // untouched — they carry over via the existing manifest plus any new
    // uploads from `upload_image`.
    if let Some(ref content) = input.content {
        let _ = input.commit_message;
        let _ = input.record;
        let content_format: ContentFormat = format.parse().unwrap_or(ContentFormat::Markdown);
        let republish = publish_article_blob(
            &state, &input.uri, &user.did, &user.token, content, content_format,
            SummaryInput { user_source: input.summary.as_deref() },
        ).await?;

        let hash = content_hash(content);
        article_service::update_article_content_hash(&state.pool, &input.uri, &hash).await?;

        if let Some(manifest) = republish.blob_manifest.clone() {
            sqlx::query("UPDATE article_localizations SET content_manifest = $1 WHERE at_uri = $2")
                .bind(&manifest)
                .bind(&input.uri)
                .execute(&state.pool)
                .await?;

            // Also putRecord the updated article shape.
            if let Some(rkey) = input.uri.rsplit('/').next() {
                // Reconstruct a CreateArticle shape from existing DB row + the new content.
                let title = input.title.clone().unwrap_or_else(|| String::new());
                let summary = input.summary.clone();
                // Build a minimal CreateArticle for record-building (only the
                // fields build_article_record actually reads).
                let create = CreateArticle {
                    title,
                    summary,
                    content: content.clone(),
                    content_format,
                    lang: None,
                    license: None,
                    translation_of: None,
                    restricted: None,
                    category: None,
                    tags: vec![],
                    prereqs: vec![],
                    related: vec![],
                    topics: vec![],
                    series_id: None,
                    metadata: None,
                    authors: vec![],
                    invites: vec![],
                    book_chapter_id: None,
                    course_session_id: None,
                };
                let record = build_article_record(&create, &manifest);
                crate::auth::pds_put_record(
                    &state, &user.token, fx_atproto::lexicon::WORK,
                    rkey.to_string(), record, "update article",
                ).await;
            }
        }
    }

    if let (Some(summary), Some(html)) = (&input.summary, &summary_html) {
        article_service::update_article_summary(&state.pool, &input.uri, summary, html).await?;
    }

    // Update category-specific metadata
    if let Some(ref meta) = input.metadata {
        match meta {
            CategoryMetadata::Paper(paper) => {
                let _ = article_service::upsert_paper_metadata(&state.pool, &input.uri, paper).await;
            }
            CategoryMetadata::Experience(exp) => {
                let _ = article_service::upsert_experience_metadata(&state.pool, &input.uri, exp).await;
            }
            CategoryMetadata::Review { book_id, edition_id, .. } => {
                // Reviews: chapter/session always NULL (review is about the
                // whole book/course).
                sqlx::query(
                    "UPDATE articles a \
                        SET book_id = $1, edition_id = $2, \
                            book_chapter_id = NULL, course_session_id = NULL \
                        FROM article_localizations l \
                      WHERE l.at_uri = $3 \
                        AND a.repo_uri = l.repo_uri AND a.source_path = l.source_path",
                )
                .bind(book_id.as_deref()).bind(edition_id.as_deref()).bind(&input.uri)
                .execute(&state.pool).await?;
            }
            CategoryMetadata::Note { book_id, edition_id, book_chapter_id, course_session_id, .. } => {
                sqlx::query(
                    "UPDATE articles a \
                        SET book_id = $1, edition_id = $2, \
                            book_chapter_id = $3, course_session_id = $4 \
                        FROM article_localizations l \
                      WHERE l.at_uri = $5 \
                        AND a.repo_uri = l.repo_uri AND a.source_path = l.source_path",
                )
                .bind(book_id.as_deref()).bind(edition_id.as_deref())
                .bind(book_chapter_id.as_deref()).bind(course_session_id.as_deref())
                .bind(&input.uri)
                .execute(&state.pool).await?;
            }
        }
    }

    // Standalone article metadata (title, summary, teaches, prereqs) lives in
    // the markdown content's YAML frontmatter — the next record operation
    // picks it up, so nothing to do here.

    let article = article_service::get_article_any_visibility(&state.pool, &input.uri).await?;
    Ok(Json(article))
}

// --- Record article change (explicit) ---

#[derive(serde::Deserialize)]
pub struct RecordArticleInput {
    pub uri: String,
    pub message: String,
}

pub async fn record_article(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<RecordArticleInput>,
) -> ApiResult<Json<Vec<version_service::ArticleVersion>>> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;

    // Version history is now an opaque byproduct of blob re-publishes: every
    // `update_article` with a new content body writes a new manifest, and
    // that IS the new version. There's no explicit "record" step any more,
    // so this endpoint degenerates to a list-versions convenience call.
    let _ = input.message;
    let versions = version_service::list_versions(&state.pool, &input.uri).await?;
    Ok(Json(versions))
}

// --- Delete article ---

#[derive(serde::Deserialize)]
pub struct DeleteArticleInput {
    pub uri: String,
}

pub async fn delete_article(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<DeleteArticleInput>,
) -> ApiResult<StatusCode> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;

    // Delete PDS records before DB delete so we can still read authorship_uri.
    let article_rkey = input.uri.rsplit('/').next().unwrap_or("").to_string();
    pds_delete_record(&state, &user.token, fx_atproto::lexicon::WORK, article_rkey, "delete article").await;

    let authorship_uri: Option<String> = sqlx::query_scalar(
        "SELECT authorship_uri FROM article_authors \
         WHERE article_uri = $1 AND author_did = $2 AND authorship_uri IS NOT NULL",
    )
    .bind(&input.uri)
    .bind(&user.did)
    .fetch_optional(&state.pool)
    .await?;
    if let Some(au) = authorship_uri {
        if let Some(rkey) = au.rsplit('/').next() {
            pds_delete_record(&state, &user.token, fx_atproto::lexicon::AUTHORSHIP, rkey.to_string(), "delete authorship").await;
        }
    }

    // Clean up the article's materialized blob cache dir.
    let node_id = uri_to_node_id(&input.uri);
    let _ = tokio::fs::remove_dir_all(state.blob_cache_path.join(&node_id)).await;

    article_service::delete_article(&state.pool, &input.uri).await?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Access control (paywall) ---

#[derive(serde::Deserialize)]
pub struct SetRestrictedInput {
    pub uri: String,
    pub restricted: bool,
}

pub async fn set_restricted(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<SetRestrictedInput>,
) -> ApiResult<StatusCode> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;
    article_service::set_restricted(&state.pool, &input.uri, input.restricted).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct AccessGrantInput {
    pub uri: String,
    pub grantee_did: String,
}

pub async fn grant_access(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<AccessGrantInput>,
) -> ApiResult<StatusCode> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;
    article_service::grant_access(&state.pool, &input.uri, &input.grantee_did).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke_access(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<AccessGrantInput>,
) -> ApiResult<StatusCode> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;
    article_service::revoke_access(&state.pool, &input.uri, &input.grantee_did).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_access_grants(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<article_service::AccessGrant>>> {
    let owner = article_service::get_article_owner(&state.pool, &uri).await?;
    require_owner(Some(&owner), &user.did)?;
    let grants = article_service::list_access_grants(&state.pool, &uri).await?;
    Ok(Json(grants))
}

// --- Search ---

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<i64>,
}

pub async fn search_articles(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let uris = fx_core::services::search_service::search_articles(&state.pool, &q.q, limit).await
        .map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))?;

    let articles = article_service::get_articles_by_uris(&state.pool, state.instance_mode, &uris).await?;
    Ok(Json(articles))
}

// --- Version history ---

pub async fn get_article_history(
    State(state): State<AppState>,
    Query(q): Query<UriQuery>,
) -> ApiResult<Json<Vec<version_service::ArticleVersion>>> {
    let versions = version_service::list_versions(&state.pool, &q.uri).await?;
    Ok(Json(versions))
}

#[derive(serde::Deserialize)]
pub struct VersionQuery {
    pub uri: String,
    pub id: i32,
}

pub async fn get_article_version(
    State(state): State<AppState>,
    Query(q): Query<VersionQuery>,
) -> ApiResult<Json<version_service::ArticleVersionFull>> {
    let version = version_service::get_version(&state.pool, &q.uri, q.id).await?;
    Ok(Json(version))
}

#[derive(serde::Deserialize)]
pub struct DiffQuery {
    pub uri: String,
    pub from: i32,
    pub to: i32,
}

pub async fn get_article_diff(
    State(state): State<AppState>,
    Query(q): Query<DiffQuery>,
) -> ApiResult<Json<version_service::VersionDiff>> {
    let diff = version_service::diff_versions(&state.pool, &q.uri, q.from, q.to).await?;
    Ok(Json(diff))
}

/// Sync a standalone markdown article's YAML frontmatter to DB after a
/// pijul change. Only applies to markdown articles — typst/html articles
/// don't have frontmatter, their metadata is DB-only.
pub(super) async fn sync_meta_to_db(state: &AppState, article_uri: &str, repo_path: &std::path::Path) {
    let content_path = repo_path.join("content.md");
    let Ok(source) = tokio::fs::read_to_string(&content_path).await else {
        return;
    };
    let (fm, _body) = fx_core::meta::split_frontmatter(&source);

    if let Some(ref title) = fm.title {
        if !title.is_empty() {
            let _ = article_service::update_article_title(&state.pool, article_uri, title).await;
        }
    }
    if let Some(ref desc) = fm.description {
        let html = crate::summary::render_summary_inline("markdown", desc, repo_path)
            .unwrap_or_default();
        let _ = article_service::update_article_summary(&state.pool, article_uri, desc, &html).await;
    }
    // Resolve the article's creator so ensure_tag can stamp created_by
    // on any brand-new label rows.
    let creator_did = article_service::get_article_owner(&state.pool, article_uri).await.unwrap_or_default();

    if !fm.teaches.is_empty() {
        let _ = sqlx::query("DELETE FROM content_teaches WHERE content_uri = $1")
            .bind(article_uri).execute(&state.pool).await;
        for input_ref in &fm.teaches {
            if let Ok(mut conn) = state.pool.acquire().await {
                if let Ok(tag_id) = fx_core::services::tag_service::resolve_tag_id(&mut conn, input_ref, &creator_did).await {
                    let _ = sqlx::query(
                        "INSERT INTO content_teaches (content_uri, tag_id) \
                         VALUES ($1, $2) ON CONFLICT DO NOTHING",
                    ).bind(article_uri).bind(&tag_id).execute(&mut *conn).await;
                }
            }
        }
    }
    if !fm.prereqs.is_empty() {
        let _ = sqlx::query("DELETE FROM content_prereqs WHERE content_uri = $1")
            .bind(article_uri).execute(&state.pool).await;
        for p in &fm.prereqs {
            if let Ok(mut conn) = state.pool.acquire().await {
                if let Ok(tag_id) = fx_core::services::tag_service::resolve_tag_id(&mut conn, &p.tag, &creator_did).await {
                    let _ = sqlx::query(
                        "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) \
                         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                    ).bind(article_uri).bind(&tag_id).bind(p.kind()).execute(&mut *conn).await;
                }
            }
        }
    }
    if !fm.related.is_empty() {
        let _ = sqlx::query("DELETE FROM content_related WHERE content_uri = $1")
            .bind(article_uri).execute(&state.pool).await;
        for input_ref in &fm.related {
            if let Ok(mut conn) = state.pool.acquire().await {
                if let Ok(tag_id) = fx_core::services::tag_service::resolve_tag_id(&mut conn, input_ref, &creator_did).await {
                    let _ = sqlx::query(
                        "INSERT INTO content_related (content_uri, tag_id) \
                         VALUES ($1, $2) ON CONFLICT DO NOTHING",
                    ).bind(article_uri).bind(&tag_id).execute(&mut *conn).await;
                }
            }
        }
    }
}

// --- Article collaboration endpoints ---

pub async fn list_article_collaborators(
    State(state): State<AppState>,
    Query(q): Query<UriQuery>,
) -> ApiResult<Json<Vec<collaboration_service::ArticleCollaborator>>> {
    let collabs = collaboration_service::list_article_collaborators(&state.pool, &q.uri).await?;
    Ok(Json(collabs))
}

#[derive(serde::Deserialize)]
pub struct InviteArticleCollabInput {
    pub uri: String,
    /// DID (did:plc:… / did:web:…) or atproto handle (e.g. alice.bsky.social).
    /// Handles are resolved server-side to a DID.
    pub identifier: String,
    pub role: Option<String>,
}

pub async fn invite_article_collaborator(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<InviteArticleCollabInput>,
) -> ApiResult<(StatusCode, Json<collaboration_service::ArticleCollaborator>)> {
    // Verify ownership
    let article = article_service::get_article(&state.pool, state.instance_mode, &input.uri).await?;
    require_owner(Some(&article.author_did), &user.did)?;

    let user_did = resolve_identifier(&state, &input.identifier).await?;

    let role = input.role.as_deref().unwrap_or("editor");
    let short_did = user_did.chars().rev().take(8).collect::<String>().chars().rev().collect::<String>();
    let channel_name = format!("collab_{short_did}");

    let collab = collaboration_service::add_article_collaborator(
        &state.pool, &input.uri, &user_did, role, &channel_name, &user.did,
    ).await?;

    Ok((StatusCode::CREATED, Json(collab)))
}

#[derive(serde::Deserialize)]
pub struct RemoveArticleCollabInput {
    pub uri: String,
    pub user_did: String,
}

pub async fn remove_article_collaborator_endpoint(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<RemoveArticleCollabInput>,
) -> ApiResult<StatusCode> {
    let article = article_service::get_article(&state.pool, state.instance_mode, &input.uri).await?;
    require_owner(Some(&article.author_did), &user.did)?;

    let removed = collaboration_service::remove_article_collaborator(&state.pool, &input.uri, &input.user_did).await?;
    if !removed {
        return Err(AppError(fx_core::Error::NotFound { entity: "collaborator", id: input.user_did }));
    }

    Ok(StatusCode::NO_CONTENT)
}
