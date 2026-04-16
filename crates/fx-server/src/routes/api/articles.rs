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
use crate::auth::{WriteAuth, pds_create_record};
use fx_core::util::{content_hash, tid, uri_to_node_id, now_rfc3339};
use super::UriQuery;

/// Look up a user's knot_url from user_settings. Returns None if not set.
pub(super) async fn get_user_knot_url(pool: &sqlx::PgPool, did: &str) -> Option<String> {
    sqlx::query_scalar::<_, Option<String>>("SELECT knot_url FROM user_settings WHERE did = $1")
        .bind(did)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .flatten()
        .filter(|u| !u.is_empty())
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

/// Shared content resolution: reads source + renders HTML for any article type
/// (independent, series chapter, heading slice, thought).
async fn resolve_article_content(
    state: &AppState,
    uri: &str,
    format: &str,
) -> Result<ArticleContent, AppError> {
    let src_ext = fx_renderer::format_extension(format);

    // Check if this is a compile-generated heading slice
    let heading_info: Option<(String, String)> = sqlx::query_as(
        "SELECT s.pijul_node_id, sa.heading_anchor \
         FROM series_articles sa JOIN series s ON s.id = sa.series_id \
         WHERE sa.article_uri = $1 AND sa.heading_anchor IS NOT NULL AND s.pijul_node_id IS NOT NULL",
    )
    .bind(uri)
    .fetch_optional(&state.pool)
    .await?;

    if let Some((node_id, anchor)) = heading_info {
        let cache_path = state.pijul.series_repo_path(&node_id)
            .join("cache")
            .join(format!("{anchor}.html"));
        let html = tokio::fs::read_to_string(&cache_path).await
            .map_err(|_| AppError(fx_core::Error::NotFound { entity: "content", id: uri.to_string() }))?;
        return Ok(ArticleContent { source: String::new(), html });
    }

    // Thoughts: content stored in DB (article_versions), not pijul
    let kind: Option<String> = sqlx::query_scalar("SELECT kind::TEXT FROM articles WHERE at_uri = $1")
        .bind(uri).fetch_optional(&state.pool).await?;
    if kind.as_deref() == Some("thought") {
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
        return Ok(ArticleContent { source, html });
    }

    // Resolve article location (unified for series and standalone)
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

/// Rewrite relative `src` and `href` attributes in HTML to point to a base URL.
/// Only rewrites paths that don't start with `/`, `http://`, `https://`, or `#`.
fn rewrite_relative_urls(html: &str, base_url: &str) -> String {
    let re = regex_lite::Regex::new(r#"(src|href)="([^"]*?)""#).unwrap();
    re.replace_all(html, |caps: &regex_lite::Captures| {
        let attr = &caps[1];
        let url = &caps[2];
        if url.starts_with("http://") || url.starts_with("https://") || url.starts_with('/') || url.starts_with('#') || url.starts_with("data:") || url.starts_with("mailto:") {
            caps[0].to_string()
        } else {
            format!("{attr}=\"{base_url}/{url}\"")
        }
    }).to_string()
}

/// Rewrite relative URLs for standalone articles: `src="foo.png"` → `src="/api/articles/image?uri=...&name=foo.png"`
fn rewrite_relative_urls_with_query(html: &str, endpoint: &str, encoded_uri: &str) -> String {
    let re = regex_lite::Regex::new(r#"(src|href)="([^"]*?)""#).unwrap();
    re.replace_all(html, |caps: &regex_lite::Captures| {
        let attr = &caps[1];
        let url = &caps[2];
        if url.starts_with("http://") || url.starts_with("https://") || url.starts_with('/') || url.starts_with('#') || url.starts_with("data:") || url.starts_with("mailto:") {
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
    /// Chapter ID within the series (TID from the article URI).
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

/// Resolve the pijul location for an article (series chapter or standalone).
/// This is the single decision point that replaces all `get_series_pijul_info` + branching.
pub(super) async fn resolve_location(
    state: &AppState,
    article_uri: &str,
    format: &str,
) -> Option<ArticleLocation> {
    let src_ext = fx_renderer::format_extension(format);
    let chapter_id = article_uri.rsplit('/').next()?;

    // Check if article belongs to a series with a pijul repo
    let series_row: Option<(String, String)> = sqlx::query_as(
        "SELECT s.pijul_node_id, s.id FROM series s \
         JOIN series_articles sa ON s.id = sa.series_id \
         WHERE sa.article_uri = $1 AND s.pijul_node_id IS NOT NULL \
         LIMIT 1",
    )
    .bind(article_uri)
    .fetch_optional(&state.pool)
    .await
    .ok()?;

    if let Some((pijul_node_id, series_id)) = series_row {
        let repo_path = state.pijul.repo_path(&pijul_node_id);
        let content_path = fx_core::meta::resolve_chapter_path(&repo_path, chapter_id, src_ext);

        // Verify chapter exists
        if !content_path.exists() {
            // Try other extensions as fallback
            let found = ["md", "typ", "html"].iter().any(|ext| {
                fx_core::meta::resolve_chapter_path(&repo_path, chapter_id, ext).exists()
            });
            if !found { return None; }
        }

        let cache_path = repo_path.join("cache").join(format!("{chapter_id}.html"));
        Some(ArticleLocation {
            node_id: pijul_node_id,
            repo_path,
            content_path,
            cache_path,
            series_id: Some(series_id),
            chapter_id: Some(chapter_id.to_string()),
        })
    } else {
        // Standalone article
        let node_id = uri_to_node_id(article_uri);
        let repo_path = state.pijul.repo_path(&node_id);
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
}


/// Try rendering this article with series-aware cross-chapter references.
///
/// For typst series: concatenate all chapters, compile as one document, split back.
/// For any format: rewrite intra-document anchor links to cross-article links.
///
/// Returns Some(html) on success, None to fall back to individual rendering.
async fn try_series_render(
    state: &crate::state::AppState,
    article_uri: &str,
    _format: &str,
) -> Option<String> {
    let result = series_service::get_series_chapters_for_render(
        &state.pool, article_uri,
    ).await;

    let (series_id, chapters) = match result {
        Ok(Some(v)) => v,
        Ok(None) => {
            tracing::debug!("article {article_uri} not in any series");
            return None;
        }
        Err(e) => {
            tracing::warn!("get_series_chapters_for_render failed: {e}");
            return None;
        }
    };

    if chapters.is_empty() {
        tracing::debug!("series {series_id} has no chapters");
        return None;
    }

    tracing::info!("series render: {series_id} with {} chapters", chapters.len());

    // Get series pijul repo for cache paths
    let series_pijul: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1 AND pijul_node_id IS NOT NULL",
    )
    .bind(&series_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some(ref pijul_node_id) = series_pijul else {
        tracing::warn!("series {series_id} has no pijul repo");
        return None;
    };

    tracing::info!("series pijul repo: {pijul_node_id}");

    let series_repo = state.pijul.series_repo_path(pijul_node_id);
    let cache_dir = series_repo.join("cache");
    let _ = tokio::fs::create_dir_all(&cache_dir).await;
    let series_cache = cache_dir.join("series.cache");

    // Check cache freshness against chapter sources in series repo
    let cache_fresh = is_series_cache_fresh(&series_cache, &chapters, &series_repo).await;

    if cache_fresh {
        let chapter_id = article_uri.rsplit('/').next().unwrap_or("");
        let ch_cache = cache_dir.join(format!("{chapter_id}.html"));
        return tokio::fs::read_to_string(&ch_cache).await.ok();
    }

    let all_typst = chapters.iter().all(|c| c.content_format == "typst");

    if all_typst {
        // Typst: virtual document compilation
        typst_series_render(state, article_uri, &series_id, &chapters, &series_cache, pijul_node_id).await
    } else {
        // Any format: render individually, then rewrite cross-chapter anchor links
        anchor_rewrite_series_render(state, article_uri, &series_id, &chapters, &series_cache).await
    }
}

async fn is_series_cache_fresh(
    series_cache: &std::path::Path,
    chapters: &[series_service::SeriesChapterInfo],
    series_repo: &std::path::Path,
) -> bool {
    let Ok(cache_meta) = tokio::fs::metadata(series_cache).await else { return false };
    let Ok(cache_time) = cache_meta.modified() else { return false };
    for ch in chapters {
        let chapter_id = ch.article_uri.rsplit('/').next().unwrap_or("");
        let ext = fx_renderer::format_extension(&ch.content_format);
        let src = fx_core::meta::resolve_chapter_path(series_repo, chapter_id, ext);
        if let Ok(src_meta) = tokio::fs::metadata(&src).await {
            if let Ok(st) = src_meta.modified() {
                if st > cache_time { return false; }
            }
        }
    }
    true
}

/// Typst series: compile the repo's main.typ which #includes all chapters.
async fn typst_series_render(
    state: &crate::state::AppState,
    article_uri: &str,
    _series_id: &str,
    chapters: &[series_service::SeriesChapterInfo],
    series_cache: &std::path::Path,
    pijul_node_id: &str,
) -> Option<String> {
    let series_repo = state.pijul.series_repo_path(pijul_node_id);

    // Build chapter_ids: (article_uri, chapter_index)
    let chapter_ids: Vec<(String, usize)> = chapters.iter().enumerate()
        .map(|(i, ch)| (ch.article_uri.clone(), i))
        .collect();

    let repo = series_repo.clone();
    let rendered = tokio::task::spawn_blocking(move || {
        let config = fx_renderer::fx_render_config();
        fx_renderer::typst_render::render_series_to_html_with_config(&chapter_ids, &repo, &config)
    }).await.ok()?;

    match rendered {
        Ok(map) => {
            write_series_cache(state, _series_id, &map, series_cache).await;
            map.get(article_uri).cloned()
        }
        Err(e) => {
            tracing::warn!("series render failed: {e}");
            None
        }
    }
}

/// Any format: render each chapter individually, then rewrite cross-chapter anchor links.
///
/// Collects all anchor IDs from all chapters, then for each chapter rewrites
/// `href="#anchor"` links to `#/article?uri=TARGET_URI` when the anchor
/// exists in a different chapter.
async fn anchor_rewrite_series_render(
    state: &crate::state::AppState,
    article_uri: &str,
    series_id: &str,
    chapters: &[series_service::SeriesChapterInfo],
    series_cache: &std::path::Path,
) -> Option<String> {
    use std::collections::HashMap;

    // Render each chapter individually
    let mut chapter_htmls: Vec<(String, String)> = Vec::new();
    for ch in chapters {
        let node = uri_to_node_id(&ch.article_uri);
        let repo = state.pijul.repo_path(&node);
        let ext = fx_renderer::format_extension(&ch.content_format);
        let src_path = repo.join(format!("content.{ext}"));
        let html_path = repo.join("content.html");

        let source = tokio::fs::read_to_string(&src_path).await.ok()?;
        let html = if ch.content_format == "html" {
            source
        } else if is_cached_fresh(&html_path, &src_path).await {
            tokio::fs::read_to_string(&html_path).await.ok()?
        } else {
            let fmt = ch.content_format.clone();
            let src = source.clone();
            let rp = repo.clone();
            let rendered = tokio::task::spawn_blocking(move || {
                let config = fx_renderer::fx_render_config();
                fx_renderer::render_to_html_with_config(&fmt, &src, &rp, &config)
            }).await.ok()?.ok()?;
            let _ = tokio::fs::write(&html_path, &rendered).await;
            rendered
        };
        chapter_htmls.push((ch.article_uri.clone(), html));
    }

    // Collect all anchor IDs from all chapters: id -> article_uri
    let mut anchor_map: HashMap<String, String> = HashMap::new();
    for (uri, html) in &chapter_htmls {
        for cap in ANCHOR_ID_RE.captures_iter(html) {
            let anchor = cap.get(1)?.as_str().to_string();
            anchor_map.entry(anchor).or_insert_with(|| uri.clone());
        }
    }

    // Rewrite cross-chapter links in each chapter
    let mut rendered: HashMap<String, String> = HashMap::new();
    for (uri, html) in &chapter_htmls {
        let rewritten = LINK_HREF_RE.replace_all(html, |caps: &regex_lite::Captures| {
            let anchor = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Some(target_uri) = anchor_map.get(anchor) {
                if target_uri != uri {
                    // Cross-chapter link: rewrite to article URL
                    return format!(
                        r##"href="#/article?uri={}#{}""##,
                        urlencoding::encode(target_uri),
                        anchor,
                    );
                }
            }
            // Same-chapter link or unknown anchor: keep as-is
            caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
        });
        rendered.insert(uri.clone(), rewritten.into_owned());
    }

    write_series_cache(state, series_id, &rendered, series_cache).await;
    rendered.get(article_uri).cloned()
}

async fn write_series_cache(
    _state: &crate::state::AppState,
    _series_id: &str,
    rendered: &std::collections::HashMap<String, String>,
    series_cache: &std::path::Path,
) {
    // Write per-chapter HTML caches to the same directory as series_cache
    let cache_dir = series_cache.parent().unwrap_or(std::path::Path::new("."));
    for (ch_uri, ch_html) in rendered {
        let chapter_id = ch_uri.rsplit('/').next().unwrap_or("");
        let ch_cache = cache_dir.join(format!("{chapter_id}.html"));
        let _ = tokio::fs::write(&ch_cache, ch_html).await;
    }
    // Touch the series cache marker
    let _ = tokio::fs::write(series_cache, "").await;
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

pub async fn get_article_prereqs(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<ArticlePrereqRow>>> {
    let prereqs = article_service::get_article_prereqs(&state.pool, &uri).await?;
    Ok(Json(prereqs))
}

pub async fn get_article_forks(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<ForkWithTitle>>> {
    let forks = article_service::get_article_forks(&state.pool, &uri).await?;
    Ok(Json(forks))
}

/// Get changes in a fork that the original article doesn't have.
pub async fn get_fork_ahead(
    State(state): State<AppState>,
    Query(q): Query<ForkAheadQuery>,
) -> ApiResult<Json<Vec<String>>> {
    let fork_node = uri_to_node_id(&q.fork_uri);
    let original_node = uri_to_node_id(&q.original_uri);

    let ahead = state.pijul.diff_repos(&fork_node, &original_node)
        .unwrap_or_default();

    Ok(Json(ahead))
}

#[derive(serde::Deserialize)]
pub struct ForkAheadQuery {
    pub fork_uri: String,
    pub original_uri: String,
}

/// Result of publishing article content: repo path + rendered HTML.
pub(super) struct PublishResult {
    pub repo_path: std::path::PathBuf,
}

/// Shared: write source file, render, record pijul, track version.
/// Callers must ensure the pijul repo is initialized before calling this.
/// Supports both local PijulStore and remote KnotClient based on user's knot_url setting.
pub(super) async fn publish_article_content(
    state: &AppState,
    at_uri: &str,
    did: &str,
    token: &str,
    content: &str,
    format: ContentFormat,
    series_id: Option<&str>,
    message: &str,
) -> Result<PublishResult, AppError> {
    let fmt = format.as_str();
    let loc = if let Some(sid) = series_id {
        // Series: resolve location from DB
        let pijul_node_id: String = sqlx::query_scalar(
            "SELECT pijul_node_id FROM series WHERE id = $1",
        )
        .bind(sid)
        .fetch_optional(&state.pool)
        .await?
        .flatten()
        .ok_or_else(|| AppError(fx_core::Error::BadRequest("Series has no pijul repo".into())))?;

        let chapter_id = at_uri.rsplit('/').next().unwrap_or("unknown");
        let repo_path = state.pijul.repo_path(&pijul_node_id);
        let src_ext = fx_renderer::format_extension(fmt);
        let content_path = fx_core::meta::resolve_chapter_path(&repo_path, chapter_id, src_ext);
        let cache_path = repo_path.join("cache").join(format!("{chapter_id}.html"));
        ArticleLocation {
            node_id: pijul_node_id,
            repo_path, content_path, cache_path,
            series_id: Some(sid.to_string()),
            chapter_id: Some(chapter_id.to_string()),
        }
    } else {
        // Standalone: derive from URI
        let node_id = uri_to_node_id(at_uri);
        let repo_path = state.pijul.repo_path(&node_id);
        let src_ext = fx_renderer::format_extension(fmt);
        ArticleLocation {
            content_path: repo_path.join(format!("content.{src_ext}")),
            cache_path: repo_path.join("content.html"),
            node_id, repo_path,
            series_id: None, chapter_id: None,
        }
    };

    let knot_url = get_user_knot_url(&state.pool, did).await;

    // Write content to knot (remote) if configured
    if let Some(ref knot) = knot_url {
        let client = pijul_knot::KnotClient::new(knot);
        let rel_path = loc.content_path.strip_prefix(&loc.repo_path)
            .unwrap_or(&loc.content_path).to_string_lossy().to_string();
        if let Err(e) = client.write_file(&loc.node_id, &rel_path, content.as_bytes()).await {
            tracing::warn!("knot write_file failed: {e}");
        }
    }

    // Always write locally (for rendering/caching)
    if let Some(parent) = loc.content_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    tokio::fs::write(&loc.content_path, content).await?;

    // Render (best-effort — may fail if resources not yet uploaded)
    if format != ContentFormat::Html {
        match render_content(fmt, content, &loc.repo_path) {
            Ok(rendered) => {
                if let Some(parent) = loc.cache_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let _ = tokio::fs::write(&loc.cache_path, &rendered).await;
            }
            Err(e) => tracing::warn!("initial render skipped (resources may be pending): {e:?}"),
        }
    }

    // Record pijul change
    let record_result = if let Some(ref knot) = knot_url {
        let client = pijul_knot::KnotClient::new(knot);
        client.record(&loc.node_id, message, Some(did)).await.ok()
            .flatten().map(|r| Ok(Some(r)))
            .unwrap_or(Ok(None))
    } else {
        state.pijul_record(loc.node_id.clone(), message.into(), Some(did.to_string())).await
    };

    match record_result {
        Ok(Some((hash, new_state))) => {
            let _ = version_service::record_version(
                &state.pool, at_uri, &hash, did, message, content,
            ).await;
            publish_pijul_ref_update(state, token, at_uri, did, &hash, &new_state).await;
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("pijul record failed for {}: {e}", loc.node_id),
    }

    Ok(PublishResult { repo_path: loc.repo_path })
}

/// Publish a sh.tangled.pijul.refUpdate record to the user's PDS.
/// Best-effort: failures are logged but do not block the request.
pub(super) async fn publish_pijul_ref_update(
    state: &AppState,
    token: &str,
    article_at_uri: &str,
    did: &str,
    change_hash: &str,
    new_state: &str,
) {
    use crate::auth::pds_create_record;
    let record = serde_json::json!({
        "$type": fx_atproto::lexicon::PIJUL_REF_UPDATE,
        "repo": article_at_uri,
        "channel": "main",
        "newState": new_state,
        "changes": [change_hash],
        "committerDid": did,
    });
    pds_create_record(state, token, fx_atproto::lexicon::PIJUL_REF_UPDATE, record, None, "pijul refUpdate").await;
}

/// Shared: write article content to the correct repo (series or independent),
/// render HTML, and optionally record a pijul change.
/// When `record` is false, only the working copy and DB are updated (no pijul change).
pub(super) async fn update_article_content(
    state: &AppState,
    uri: &str,
    did: &str,
    token: Option<&str>,
    content: &str,
    format: &str,
    message: &str,
) -> Result<(), AppError> {
    save_article_content(state, uri, content, format).await?;
    record_pijul_change(state, uri, did, token, content, message).await?;
    Ok(())
}

/// Write article content to working copy and render HTML, without recording a pijul change.
pub(super) async fn save_article_content(
    state: &AppState,
    uri: &str,
    content: &str,
    format: &str,
) -> Result<(), AppError> {
    let loc = resolve_location(state, uri, format).await
        .ok_or(AppError(fx_core::Error::NotFound { entity: "article", id: uri.to_string() }))?;

    if let Some(parent) = loc.content_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    tokio::fs::write(&loc.content_path, content).await?;

    if format != "html" {
        match render_content(format, content, &loc.repo_path) {
            Ok(rendered) => {
                if let Some(parent) = loc.cache_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let _ = tokio::fs::write(&loc.cache_path, &rendered).await;
            }
            Err(e) => tracing::warn!("render failed: {e:?}"),
        }
        if loc.is_series() {
            let _ = tokio::fs::remove_file(loc.repo_path.join("cache").join("series.cache")).await;
        }
    }

    let hash = content_hash(content);
    article_service::update_article_content_hash(&state.pool, uri, &hash).await?;
    Ok(())
}

/// Record the current working copy state as a pijul change and store version metadata.
pub(super) async fn record_pijul_change(
    state: &AppState,
    uri: &str,
    did: &str,
    token: Option<&str>,
    content: &str,
    message: &str,
) -> Result<(), AppError> {
    let loc = resolve_location(state, uri, "typst").await
        .ok_or(AppError(fx_core::Error::NotFound { entity: "article", id: uri.to_string() }))?;
    let knot_url = get_user_knot_url(&state.pool, did).await;

    let record_result = if let Some(ref knot) = knot_url {
        let client = pijul_knot::KnotClient::new(knot);
        client.record(&loc.node_id, message, Some(did)).await.ok()
            .flatten().map(|r| Ok(Some(r)))
            .unwrap_or(Ok(None))
    } else {
        state.pijul_record(loc.node_id.clone(), message.into(), Some(did.to_string())).await
    };

    match record_result {
        Ok(Some((hash, new_state))) => {
            let _ = version_service::record_version(
                &state.pool, uri, &hash, did, message, content,
            ).await;
            if let Some(tok) = token {
                publish_pijul_ref_update(state, tok, uri, did, &hash, &new_state).await;
            }
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("pijul record failed for {}: {e}", loc.node_id),
    }
    Ok(())
}

pub async fn create_article(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateArticle>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    validate_create_article(&input)?;

    let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::ARTICLE, tid());

    // Verify series ownership if series_id is specified
    if let Some(ref sid) = input.series_id {
        let owner = series_service::get_series_owner(&state.pool, sid).await?;
        require_owner(Some(&owner), &user.did)?;
    }

    // Init pijul repo for standalone articles (series repos are already initialized)
    if input.series_id.is_none() {
        let node_id = uri_to_node_id(&at_uri);
        let knot_url = get_user_knot_url(&state.pool, &user.did).await;
        if let Some(ref knot) = knot_url {
            let client = pijul_knot::KnotClient::new(knot);
            if let Err(e) = client.init_repo(&node_id).await {
                tracing::warn!("knot init_repo failed: {e}");
            }
        }
        state.pijul.init_repo(&node_id)
            .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;
    }

    let publish = publish_article_content(
        &state, &at_uri, &user.did, &user.token, &input.content, input.content_format,
        input.series_id.as_deref(), "Initial publish",
    ).await?;

    // Write meta.json to pijul repo (for standalone articles)
    if input.series_id.is_none() {
        let meta = fx_core::meta::article_meta_from_create(
            &input.title,
            input.description.as_deref(),
            &input.tags,
            &input.prereqs,
            input.license.as_deref(),
            input.lang.as_deref(),
            input.category.as_deref(),
            input.content_format.as_str(),
        );
        if let Err(e) = fx_core::meta::write_meta_file(&publish.repo_path, &meta) {
            tracing::warn!("failed to write meta.json: {e}");
        }
        let node_id = uri_to_node_id(&at_uri);
        let _ = state.pijul_record(node_id.clone(), "Add metadata".into(), Some(user.did.clone())).await;
    }

    let hash = content_hash(&input.content);

    let translation_group = if let Some(ref source_uri) = input.translation_of {
        Some(article_service::resolve_translation_group(&state.pool, source_uri).await?)
    } else {
        None
    };

    let article = article_service::create_article(
        &state.pool, &user.did, &at_uri, &input, &hash, translation_group,
        default_visibility(user.phone_verified), ContentKind::Article, None,
    ).await?;

    // Add to series if specified
    if let Some(ref sid) = input.series_id {
        series_service::add_series_article(&state.pool, sid, &at_uri).await?;
    }

    // Skip PDS sync for restricted articles — content stays server-local only
    if !input.restricted.unwrap_or(false) {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::ARTICLE,
            "title": input.title,
            "description": input.description.as_deref().unwrap_or(""),
            "contentFormat": input.content_format,
            "tags": input.tags,
            "createdAt": now_rfc3339(),
        });
        pds_create_record(&state, &user.token, fx_atproto::lexicon::ARTICLE, record, None, "create article").await;
    }

    let _ = article_service::auto_bookmark(&state.pool, &user.did, &at_uri).await;

    // Register creator as owner collaborator
    let _ = collaboration_service::register_article_owner(&state.pool, &at_uri, &user.did).await;

    // Save paper metadata if provided
    if let Some(ref paper) = input.paper {
        let _ = article_service::upsert_paper_metadata(&state.pool, &at_uri, paper).await;
    }

    // Add creator as verified author (position 0) with PDS authorship record
    let authorship_record = serde_json::json!({
        "$type": fx_atproto::lexicon::AUTHORSHIP,
        "article": at_uri,
        "createdAt": now_rfc3339(),
    });
    let authorship_uri = pds_create_record(
        &state, &user.token, fx_atproto::lexicon::AUTHORSHIP, authorship_record, None, "creator authorship",
    ).await;
    let _ = authorship_service::add_author(&state.pool, &at_uri, &user.did, &user.did, Some(0)).await;
    if let Some(ref uri) = authorship_uri {
        let _ = authorship_service::verify_authorship(&state.pool, &at_uri, &user.did, Some(uri)).await;
    }

    // Add co-authors as pending (they must verify)
    for (i, author_did) in input.authors.iter().enumerate() {
        if author_did != &user.did {
            let _ = authorship_service::add_author(
                &state.pool, &at_uri, author_did, &user.did, Some((i + 1) as i16),
            ).await;
        }
    }

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

    let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::ARTICLE, tid());

    if let Some(ref sid) = input.series_id {
        let owner = series_service::get_series_owner(&state.pool, sid).await?;
        require_owner(Some(&owner), &user.did)?;
    }

    // Init repo for standalone articles (series repos already exist)
    if input.series_id.is_none() {
        let node_id = uri_to_node_id(&at_uri);
        let knot_url = get_user_knot_url(&state.pool, &user.did).await;
        if let Some(ref knot) = knot_url {
            let client = pijul_knot::KnotClient::new(knot);
            if let Err(e) = client.init_repo(&node_id).await {
                tracing::warn!("knot init_repo failed: {e}");
            }
        }
        state.pijul.init_repo(&node_id)
            .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;

        // Write resources BEFORE publishing content (so rendering can find them)
        if !resources.is_empty() {
            let repo_path = state.pijul.repo_path(&node_id);
            for (rel_path, data) in &resources {
                let safe_path: String = rel_path.chars()
                    .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' { c } else { '_' })
                    .collect();
                let safe_path = safe_path.trim_start_matches('.').trim_start_matches('/');
                if safe_path.is_empty() || safe_path.contains("..") {
                    continue;
                }
                let dest = repo_path.join(safe_path);
                if let Some(parent) = dest.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(&dest, data).await?;
            }
            let _ = state.pijul_record(node_id, format!("Upload {} resources", resources.len()), Some(user.did.clone())).await;
        }
    }

    // Now publish content — resources are already in the repo, so rendering will work
    let publish = publish_article_content(
        &state, &at_uri, &user.did, &user.token, &input.content, input.content_format,
        input.series_id.as_deref(), "Initial publish",
    ).await?;

    if input.series_id.is_none() {
        let meta = fx_core::meta::article_meta_from_create(
            &input.title,
            input.description.as_deref(),
            &input.tags,
            &input.prereqs,
            input.license.as_deref(),
            input.lang.as_deref(),
            input.category.as_deref(),
            input.content_format.as_str(),
        );
        if let Err(e) = fx_core::meta::write_meta_file(&publish.repo_path, &meta) {
            tracing::warn!("failed to write meta.json: {e}");
        }
        let node_id = uri_to_node_id(&at_uri);
        let _ = state.pijul_record(node_id.clone(), "Add metadata".into(), Some(user.did.clone())).await;
    }

    let hash = content_hash(&input.content);

    let translation_group = if let Some(ref source_uri) = input.translation_of {
        Some(article_service::resolve_translation_group(&state.pool, source_uri).await?)
    } else {
        None
    };

    let article = article_service::create_article(
        &state.pool, &user.did, &at_uri, &input, &hash, translation_group,
        default_visibility(user.phone_verified), ContentKind::Article, None,
    ).await?;

    if let Some(ref sid) = input.series_id {
        series_service::add_series_article(&state.pool, sid, &at_uri).await?;
    }

    if !input.restricted.unwrap_or(false) {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::ARTICLE,
            "title": input.title,
            "description": input.description.as_deref().unwrap_or(""),
            "contentFormat": input.content_format,
            "tags": input.tags,
            "createdAt": now_rfc3339(),
        });
        pds_create_record(&state, &user.token, fx_atproto::lexicon::ARTICLE, record, None, "create article").await;
    }

    let _ = article_service::auto_bookmark(&state.pool, &user.did, &at_uri).await;
    let _ = collaboration_service::register_article_owner(&state.pool, &at_uri, &user.did).await;

    if let Some(ref paper) = input.paper {
        let _ = article_service::upsert_paper_metadata(&state.pool, &at_uri, paper).await;
    }

    let authorship_record = serde_json::json!({
        "$type": fx_atproto::lexicon::AUTHORSHIP,
        "article": at_uri,
        "createdAt": now_rfc3339(),
    });
    let authorship_uri = pds_create_record(
        &state, &user.token, fx_atproto::lexicon::AUTHORSHIP, authorship_record, None, "creator authorship",
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

// --- Full article page data (single request) ---

#[derive(serde::Serialize)]
pub struct ArticleFullResponse {
    article: Article,
    content: ArticleContent,
    prereqs: Vec<ArticlePrereqRow>,
    forks: Vec<ForkWithTitle>,
    fork_source: Option<String>,
    votes: ArticleVoteSummary,
    series_context: Vec<fx_core::services::series_service::SeriesContextItem>,
    translations: Vec<Article>,
    #[serde(skip_serializing_if = "Option::is_none")]
    paper: Option<PaperMetadata>,
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
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<ArticleFullResponse>> {
    use fx_core::services::{vote_service, bookmark_service, series_service, learned_service};

    let mode = state.instance_mode;
    let (article, prereqs, forks, vote_summary, series_ctx, translations) = tokio::try_join!(
        article_service::get_article(&state.pool, mode, &uri),
        article_service::get_article_prereqs(&state.pool, &uri),
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

    Ok(Json(ArticleFullResponse {
        article,
        content,
        prereqs,
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
    let articles = article_service::get_articles_by_tag(&state.pool, state.instance_mode, &q.tag_id, limit).await?;
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

// --- Fork ---

pub async fn fork_article(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ForkArticleInput>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    if let Err(e) = fx_core::validation::validate_at_uri(&input.uri) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    let source = article_service::get_article(&state.pool, state.instance_mode, &input.uri).await?;

    if source.license == "All-Rights-Reserved" {
        return Err(AppError(fx_core::Error::Forbidden { action: "fork proprietary article" }));
    }
    if source.license.contains("ND") {
        return Err(AppError(fx_core::Error::Forbidden { action: "fork NoDerivatives-licensed article" }));
    }
    if source.restricted {
        return Err(AppError(fx_core::Error::Forbidden { action: "fork restricted article" }));
    }

    let fork_at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::ARTICLE, tid());
    let fork_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::FORK, tid());

    let source_node_id = uri_to_node_id(&input.uri);
    let fork_node_id = uri_to_node_id(&fork_at_uri);
    state.pijul.fork(&source_node_id, &fork_node_id)
        .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;

    // Format conversion on fork
    let target_format = input.target_format.as_deref().unwrap_or(source.content_format.as_str());
    if target_format != source.content_format.as_str() {
        if let Err(e) = fx_core::validation::validate_content_format(target_format) {
            return Err(AppError(fx_core::Error::Validation(vec![e])));
        }
        let fork_repo = state.pijul.repo_path(&fork_node_id);
        let old_ext = fx_renderer::format_extension(source.content_format.as_str());
        let new_ext = fx_renderer::format_extension(target_format);
        let old_path = fork_repo.join(format!("content.{old_ext}"));
        let src_content = tokio::fs::read_to_string(&old_path).await?;
        let converted = fx_renderer::convert_format(&src_content, source.content_format.as_str(), target_format)
            .map_err(|e| AppError(fx_core::Error::Render(e.to_string())))?;
        // Write new format file and remove old one
        let new_path = fork_repo.join(format!("content.{new_ext}"));
        tokio::fs::write(&new_path, &converted).await?;
        if old_ext != new_ext {
            let _ = tokio::fs::remove_file(&old_path).await;
        }
        // Re-render HTML cache
        if target_format != "html" {
            let rendered = render_content(target_format, &converted, &fork_repo)?;
            let _ = tokio::fs::write(fork_repo.join("content.html"), &rendered).await;
        }
    }

    // Record initial version for the fork
    let fork_repo = state.pijul.repo_path(&fork_node_id);
    let fork_src_ext = fx_renderer::format_extension(target_format);
    let fork_src_file = format!("content.{fork_src_ext}");
    if let Ok(source_text) = tokio::fs::read_to_string(&fork_repo.join(&fork_src_file)).await {
        let _ = version_service::record_version(
            &state.pool, &fork_at_uri, "fork-initial", &user.did,
            &format!("Forked from {}", input.uri), &source_text,
        ).await;
    }

    let mut source_for_fork = source.clone();
    if target_format != source_for_fork.content_format.as_str() {
        source_for_fork.content_format = target_format.parse::<ContentFormat>()
            .map_err(|e| AppError(fx_core::Error::BadRequest(e)))?;
    }

    let article = article_service::create_fork_record(
        &state.pool, &fork_uri, &input.uri, &fork_at_uri, &user.did, &source_for_fork,
        default_visibility(user.phone_verified),
    ).await?;

    let record = serde_json::json!({
        "$type": fx_atproto::lexicon::FORK,
        "source": input.uri,
        "fork": fork_at_uri,
        "createdAt": now_rfc3339(),
    });
    pds_create_record(&state, &user.token, fx_atproto::lexicon::FORK, record, None, "create fork").await;

    if let Err(e) = notification_service::create_notification(
        &state.pool, &tid(), &source.did, &user.did,
        "article_fork", Some(&input.uri), Some(&fork_at_uri),
    ).await {
        tracing::warn!("notification failed: {e}");
    }

    Ok((StatusCode::CREATED, Json(article)))
}

#[derive(serde::Deserialize)]
pub struct ForkArticleInput {
    uri: String,
    /// Target format for format conversion on fork. If omitted, keeps original format.
    target_format: Option<String>,
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

    // Resolve article location (unified for series and standalone)
    let loc = resolve_location(&state, &uri, "typst").await
        .unwrap_or_else(|| {
            // Fallback for standalone articles without existing repo
            let node_id = uri_to_node_id(&uri);
            let repo_path = state.pijul.repo_path(&node_id);
            ArticleLocation {
                node_id, repo_path: repo_path.clone(),
                content_path: repo_path.join("content.typ"),
                cache_path: repo_path.join("content.html"),
                series_id: None, chapter_id: None,
            }
        });

    // Write resource to repo
    let dest = loc.repo_path.join(&safe_name);
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&dest, &data).await?;

    // Invalidate cache
    loc.invalidate_cache().await;

    // Record pijul change
    match state.pijul_record(loc.node_id.clone(), format!("Add resource: {safe_name}"), Some(user.did.clone())).await {
        Ok(Some((hash, new_state))) => {
            let _ = version_service::record_version(
                &state.pool, &uri, &hash, &user.did, &format!("Add resource: {safe_name}"), "",
            ).await;
            publish_pijul_ref_update(&state, &user.token, &uri, &user.did, &hash, &new_state).await;
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("pijul record failed for {}: {e}", loc.node_id),
    }

    Ok(Json(ImageUploadResponse { filename: safe_name }))
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
    let repo_path = state.pijul.repo_path(&node_id);

    // Sanitize: allow subdirectories (e.g. _rendered/hash.png, Figure/img.pdf) but reject ..
    let name = &q.name;
    if name.is_empty() || name.contains("..") || name.starts_with('/') {
        return Err(AppError(fx_core::Error::BadRequest("invalid file name".into())));
    }

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
    pub description: Option<String>,
    pub content: Option<String>,
    pub commit_message: Option<String>,
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
    if let Some(ref desc) = input.description {
        article_service::update_article_description(&state.pool, &input.uri, desc).await?;
    }

    if let Some(ref content) = input.content {
        let format = article_service::get_content_format(&state.pool, &input.uri).await?;
        if input.record {
            let msg = input.commit_message.as_deref().unwrap_or("Update article");
            update_article_content(&state, &input.uri, &user.did, Some(&user.token), content, &format, msg).await?;
        } else {
            save_article_content(&state, &input.uri, content, &format).await?;
        }
    }

    // Update meta.json if title or description changed
    if input.title.is_some() || input.description.is_some() {
        let article_for_meta = article_service::get_article_any_visibility(&state.pool, &input.uri).await?;
        let node_id = uri_to_node_id(&input.uri);
        let repo_path = state.pijul.repo_path(&node_id);
        if repo_path.exists() {
            let mut meta = fx_core::meta::read_meta_file(&repo_path).unwrap_or_default();
            meta.title = article_for_meta.title.clone();
            meta.description = if article_for_meta.description.is_empty() { None } else { Some(article_for_meta.description.clone()) };
            let _ = fx_core::meta::write_meta_file(&repo_path, &meta);
            // Don't record separately — it'll be included in the next content record
        }
    }

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
) -> ApiResult<Json<Vec<ArticleVersionInfo>>> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;

    // Read current content from working copy
    let format = article_service::get_content_format(&state.pool, &input.uri).await?;
    let loc = resolve_location(&state, &input.uri, &format).await
        .ok_or(AppError(fx_core::Error::NotFound { entity: "article", id: input.uri.clone() }))?;

    let content = tokio::fs::read_to_string(&loc.content_path).await
        .map_err(|e| AppError(fx_core::Error::Internal(format!("read working copy: {e}"))))?;

    let msg = if input.message.trim().is_empty() { "Update" } else { input.message.trim() };
    record_pijul_change(&state, &input.uri, &user.did, Some(&user.token), &content, msg).await?;

    // Return updated history
    let versions = version_service::list_versions(&state.pool, &input.uri).await?;
    let node_id = uri_to_node_id(&input.uri);
    let result = versions
        .into_iter()
        .map(|v| {
            let unrecordable = state.pijul.is_unrecordable(&node_id, &v.change_hash);
            ArticleVersionInfo { version: v, unrecordable }
        })
        .collect();
    Ok(Json(result))
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

    // Clean up source files before deleting DB record
    if let Some(loc) = resolve_location(&state, &input.uri, "typst").await {
        if loc.is_series() {
            // Remove chapter files and cache from series repo (don't delete the whole repo)
            let _ = tokio::fs::remove_file(&loc.content_path).await;
            loc.invalidate_cache().await;
        } else {
            // Remove entire standalone repo
            let _ = tokio::fs::remove_dir_all(&loc.repo_path).await;
        }
    }

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
    let engine = fx_search::SearchEngine::new(state.pool.clone());
    let uris = engine.search(&q.q, limit).await
        .map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))?;

    let articles = article_service::get_articles_by_uris(&state.pool, state.instance_mode, &uris).await?;
    Ok(Json(articles))
}

// --- Version history ---

#[derive(serde::Serialize)]
pub struct ArticleVersionInfo {
    #[serde(flatten)]
    pub version: version_service::ArticleVersion,
    pub unrecordable: bool,
}

pub async fn get_article_history(
    State(state): State<AppState>,
    Query(q): Query<UriQuery>,
) -> ApiResult<Json<Vec<ArticleVersionInfo>>> {
    let versions = version_service::list_versions(&state.pool, &q.uri).await?;
    let node_id = uri_to_node_id(&q.uri);
    let result = versions
        .into_iter()
        .map(|v| {
            let unrecordable = state.pijul.is_unrecordable(&node_id, &v.change_hash);
            ArticleVersionInfo { version: v, unrecordable }
        })
        .collect();
    Ok(Json(result))
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

// --- Unrecord the latest change for an article ---

#[derive(serde::Deserialize)]
pub struct UnrecordInput {
    pub uri: String,
    pub version_id: i32,
}

pub async fn unrecord_article_change(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UnrecordInput>,
) -> ApiResult<StatusCode> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;

    let version = version_service::get_version(&state.pool, &input.uri, input.version_id).await?;

    // Perform pijul unrecord — fails with BadRequest if other changes depend on this one
    let node_id = uri_to_node_id(&input.uri);
    state.pijul.unrecord_change(&node_id, &version.change_hash)
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("depended upon") {
                AppError(fx_core::Error::BadRequest(
                    "该修改有后继依赖，无法撤销".into(),
                ))
            } else {
                AppError(fx_core::Error::Internal(msg))
            }
        })?;

    // Read the restored content from the working copy
    let src_ext = article_service::get_content_format(&state.pool, &input.uri).await?;
    let ext = fx_renderer::format_extension(&src_ext);
    let content = state.pijul.get_file_content(&node_id, &format!("content.{ext}"))
        .map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))?;
    let content_str = String::from_utf8_lossy(&content).to_string();

    // Re-render and update DB content hash
    let repo_path = state.pijul.repo_path(&node_id);
    if src_ext != "html" {
        if let Ok(rendered) = render_content(&src_ext, &content_str, &repo_path) {
            let _ = tokio::fs::write(repo_path.join("content.html"), rendered).await;
        }
    }
    let hash = content_hash(&content_str);
    article_service::update_article_content_hash(&state.pool, &input.uri, &hash).await?;

    // Remove the version row
    version_service::delete_version(&state.pool, input.version_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Cross-fork apply ---

#[derive(serde::Deserialize)]
pub struct ApplyChangeInput {
    pub source_uri: String,
    pub target_uri: String,
    pub change_hash: String,
}

#[derive(serde::Serialize)]
pub struct ApplyChangeOutput {
    pub has_conflicts: bool,
    pub content: String,
}

/// Apply a pijul change from one article repo (typically a fork) to another
/// (typically the original). The target article owner must be the caller.
pub async fn apply_change(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ApplyChangeInput>,
) -> ApiResult<Json<ApplyChangeOutput>> {
    // Only the target article owner can apply changes to it.
    let owner = article_service::get_article_owner(&state.pool, &input.target_uri).await?;
    require_owner(Some(&owner), &user.did)?;

    let source_node_id = uri_to_node_id(&input.source_uri);
    let target_node_id = uri_to_node_id(&input.target_uri);

    // Apply the change (copies change file + dependencies, then applies).
    state.pijul.apply(&source_node_id, &target_node_id, &input.change_hash)
        .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;

    // Read the updated working copy.
    let src_ext = article_service::get_content_format(&state.pool, &input.target_uri).await?;
    let ext = fx_renderer::format_extension(&src_ext);
    let content_bytes = state.pijul.get_file_content(&target_node_id, &format!("content.{ext}"))
        .map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))?;
    let content = String::from_utf8_lossy(&content_bytes).to_string();

    // Check for pijul conflict markers.
    let has_conflicts = content.contains(">>>>>>>") || content.contains("<<<<<<<");

    // Re-render HTML cache.
    let repo_path = state.pijul.repo_path(&target_node_id);
    if src_ext != "html" {
        if let Ok(rendered) = render_content(&src_ext, &content, &repo_path) {
            let _ = tokio::fs::write(repo_path.join("content.html"), rendered).await;
        }
    }

    // Record a new version if no conflicts (conflicts need manual resolution first).
    if !has_conflicts {
        let message = format!("Applied change {} from {}", &input.change_hash[..12.min(input.change_hash.len())], &input.source_uri);
        // Record the apply as a pijul change.
        if let Some((hash, _merkle)) = state.pijul_record(target_node_id.clone(), message.clone(), Some(user.did.clone())).await
            .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))? {
            version_service::record_version(
                &state.pool, &input.target_uri, &hash, &user.did, &message, &content,
            ).await?;
        }

        let hash = content_hash(&content);
        article_service::update_article_content_hash(&state.pool, &input.target_uri, &hash).await?;

        // Sync meta.json → DB if it exists
        sync_meta_to_db(&state, &input.target_uri, &repo_path).await;
    }

    Ok(Json(ApplyChangeOutput { has_conflicts, content }))
}

/// Sync meta.json from pijul repo to DB after applying changes.
pub(super) async fn sync_meta_to_db(state: &AppState, article_uri: &str, repo_path: &std::path::Path) {
    let meta = match fx_core::meta::read_meta_file(repo_path) {
        Some(m) => m,
        None => return,
    };

    // Sync title
    if !meta.title.is_empty() {
        let _ = article_service::update_article_title(&state.pool, article_uri, &meta.title).await;
    }
    // Sync description
    if let Some(ref desc) = meta.description {
        let _ = article_service::update_article_description(&state.pool, article_uri, desc).await;
    }
    // Sync tags: clear old, insert new
    if !meta.tags.is_empty() {
        let _ = sqlx::query("DELETE FROM content_teaches WHERE content_uri = $1")
            .bind(article_uri)
            .execute(&state.pool)
            .await;
        for tag_id in &meta.tags {
            let _ = sqlx::query(
                "INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(article_uri)
            .bind(tag_id)
            .execute(&state.pool)
            .await;
        }
    }
    // Sync prereqs: clear old, insert new
    if !meta.prereqs.is_empty() {
        let _ = sqlx::query("DELETE FROM content_prereqs WHERE content_uri = $1")
            .bind(article_uri)
            .execute(&state.pool)
            .await;
        for p in &meta.prereqs {
            let _ = sqlx::query(
                "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(article_uri)
            .bind(&p.tag_id)
            .bind(&p.prereq_type)
            .execute(&state.pool)
            .await;
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
    pub user_did: String,
    pub role: Option<String>,
}

pub async fn invite_article_collaborator(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<InviteArticleCollabInput>,
) -> ApiResult<(StatusCode, Json<collaboration_service::ArticleCollaborator>)> {
    // Verify ownership
    let article = article_service::get_article(&state.pool, state.instance_mode, &input.uri).await?;
    require_owner(Some(&article.did), &user.did)?;

    let role = input.role.as_deref().unwrap_or("editor");
    let short_did = input.user_did.chars().rev().take(8).collect::<String>().chars().rev().collect::<String>();
    let channel_name = format!("collab_{short_did}");

    // Create pijul channel
    let node_id = uri_to_node_id(&input.uri);
    if let Err(e) = state.pijul.create_channel(&node_id, &channel_name, None) {
        tracing::warn!("create channel for article: {e}");
    }

    let collab = collaboration_service::add_article_collaborator(
        &state.pool, &input.uri, &input.user_did, role, &channel_name, &user.did,
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
    require_owner(Some(&article.did), &user.did)?;

    let collab = collaboration_service::get_article_collaborator(&state.pool, &input.uri, &input.user_did).await?;
    let removed = collaboration_service::remove_article_collaborator(&state.pool, &input.uri, &input.user_did).await?;
    if !removed {
        return Err(AppError(fx_core::Error::NotFound { entity: "collaborator", id: input.user_did }));
    }

    if let Some(c) = collab {
        let node_id = uri_to_node_id(&input.uri);
        let _ = state.pijul.delete_channel(&node_id, &c.channel_name);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_article_channels(
    State(state): State<AppState>,
    Query(q): Query<UriQuery>,
) -> ApiResult<Json<Vec<String>>> {
    let node_id = uri_to_node_id(&q.uri);
    let channels = state.pijul.list_channels(&node_id)
        .unwrap_or_else(|_| vec!["main".to_string()]);
    Ok(Json(channels))
}

#[derive(serde::Deserialize)]
pub struct ArticleChannelFileQuery {
    pub uri: String,
    pub channel: String,
    pub path: Option<String>,
}

pub async fn read_article_channel_file(
    State(state): State<AppState>,
    Query(q): Query<ArticleChannelFileQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let node_id = uri_to_node_id(&q.uri);
    let file_path = q.path.as_deref().unwrap_or("content.typ");

    let content = state.pijul.read_file_from_channel(&node_id, &q.channel, file_path)
        .map_err(|e| AppError(fx_core::Error::Internal(format!("read channel file: {e}"))))?;

    let text = String::from_utf8_lossy(&content).into_owned();
    Ok(Json(serde_json::json!({ "content": text })))
}

#[derive(serde::Deserialize)]
pub struct WriteArticleChannelFileInput {
    pub uri: String,
    pub channel: String,
    pub content: String,
    pub message: Option<String>,
    pub path: Option<String>,
}

pub async fn write_article_channel_file(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<WriteArticleChannelFileInput>,
) -> ApiResult<Json<serde_json::Value>> {
    // Verify user has access to this channel
    let collab = collaboration_service::get_article_collaborator(&state.pool, &input.uri, &user.did).await?;
    let allowed = match &collab {
        Some(c) => c.channel_name == input.channel || c.role == "owner",
        None => false,
    };
    if !allowed {
        return Err(AppError(fx_core::Error::Forbidden { action: "write to this channel" }));
    }

    let node_id = uri_to_node_id(&input.uri);
    let file_path = input.path.as_deref().unwrap_or("content.typ");
    let msg = input.message.as_deref().unwrap_or("update");

    let result = state.pijul.write_and_record_on_channel(
        &node_id, &input.channel, file_path, input.content.as_bytes(), msg, Some(&user.did),
    ).map_err(|e| AppError(fx_core::Error::Internal(format!("write channel file: {e}"))))?;

    let (hash, merkle) = result.unwrap_or_default();
    Ok(Json(serde_json::json!({ "change_hash": hash, "merkle": merkle })))
}

#[derive(serde::Deserialize)]
pub struct ArticleChannelQuery {
    pub uri: String,
    pub channel: String,
}

pub async fn article_channel_log(
    State(state): State<AppState>,
    Query(q): Query<ArticleChannelQuery>,
) -> ApiResult<Json<Vec<String>>> {
    let node_id = uri_to_node_id(&q.uri);
    let log = state.pijul.log_channel(&node_id, &q.channel)
        .map_err(|e| AppError(fx_core::Error::Internal(format!("channel log: {e}"))))?;
    Ok(Json(log))
}

#[derive(serde::Deserialize)]
pub struct ApplyArticleChannelInput {
    pub uri: String,
    pub target_channel: String,
    pub change_hash: String,
}

pub async fn apply_article_channel_change(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ApplyArticleChannelInput>,
) -> ApiResult<StatusCode> {
    let collab = collaboration_service::get_article_collaborator(&state.pool, &input.uri, &user.did).await?;
    let allowed = match &collab {
        Some(c) => c.channel_name == input.target_channel || c.role == "owner",
        None => false,
    };
    if !allowed {
        return Err(AppError(fx_core::Error::Forbidden { action: "write to this channel" }));
    }

    let node_id = uri_to_node_id(&input.uri);
    state.pijul.apply_change_to_channel(&node_id, &input.change_hash, &input.target_channel)
        .map_err(|e| AppError(fx_core::Error::Internal(format!("apply change: {e}"))))?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct ArticleChannelDiffQuery {
    pub uri: String,
    pub a: String,
    pub b: String,
}

pub async fn article_channel_diff(
    State(state): State<AppState>,
    Query(q): Query<ArticleChannelDiffQuery>,
) -> ApiResult<Json<pijul_knot::ChannelDiffResult>> {
    let node_id = uri_to_node_id(&q.uri);
    let diff = state.pijul.diff_channels(&node_id, &q.a, &q.b)
        .map_err(|e| AppError(fx_core::Error::Internal(format!("channel diff: {e}"))))?;
    Ok(Json(diff))
}
