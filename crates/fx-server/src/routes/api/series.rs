use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
};
use fx_core::content::{ContentFormat, ContentKind};
use fx_core::models::*;
use fx_core::services::{article_service, collaboration_service, series_service};
use fx_core::validation;

use crate::error::{AppError, ApiResult, require_owner};
use crate::state::AppState;
use crate::auth::WriteAuth;
use fx_core::util::tid;
use super::UriQuery;

#[derive(serde::Deserialize)]
pub(crate) struct CreateSeriesInput {
    title: String,
    description: Option<String>,
    long_description: Option<String>,
    topics: Option<Vec<String>>,
    lang: Option<String>,
    translation_of: Option<String>,
    category: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct ListSeriesQuery {
    pub limit: Option<i64>,
}

pub async fn list_series(
    State(state): State<AppState>,
    Query(q): Query<ListSeriesQuery>,
) -> ApiResult<Json<Vec<series_service::SeriesListRow>>> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let rows = series_service::list_series(&state.pool, limit).await?;
    Ok(Json(rows))
}

pub async fn create_series(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateSeriesInput>,
) -> ApiResult<(StatusCode, Json<series_service::SeriesRow>)> {
    if let Err(e) = validation::validate_title(&input.title) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    let id = format!("s-{}", tid());
    let topics = input.topics.unwrap_or_default();
    let lang = input.lang.as_deref().unwrap_or("zh");
    let translation_group = if let Some(ref source_id) = input.translation_of {
        Some(series_service::resolve_series_translation_group(&state.pool, source_id).await?)
    } else {
        None
    };

    let category = input.category.as_deref().unwrap_or("general");
    // Initialize pijul repo for all series (articles stored in series repo)
    let node_id = format!("series_{id}");
    if let Err(e) = state.pijul.init_series_repo(&node_id) {
        tracing::warn!("failed to init series pijul repo: {e}");
    }
    let pijul_node_id = Some(node_id);

    let row = series_service::create_series(
        &state.pool,
        &id,
        &input.title,
        input.description.as_deref(),
        input.long_description.as_deref(),
        &topics,
        &user.did,
        lang,
        translation_group,
        category,
        pijul_node_id.as_deref(),
    )
    .await?;

    // Register creator as owner collaborator
    let _ = collaboration_service::register_owner(&state.pool, &id, &user.did).await;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn get_series_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<series_service::SeriesDetailResponse>> {
    let detail = series_service::get_series_detail(&state.pool, &id).await?;
    Ok(Json(detail))
}

#[derive(serde::Deserialize)]
pub(crate) struct AddSeriesArticleInput {
    article_uri: String,
}

pub async fn add_series_article(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<AddSeriesArticleInput>,
) -> ApiResult<StatusCode> {
    if let Err(e) = validation::validate_at_uri(&input.article_uri) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    let owner = series_service::get_series_owner(&state.pool, &id).await?;
    require_owner(Some(&owner), &user.did)?;

    series_service::add_series_article(&state.pool, &id, &input.article_uri).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct RemoveSeriesArticleInput {
    article_uri: String,
}

pub async fn remove_series_article(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<RemoveSeriesArticleInput>,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &id).await?;
    require_owner(Some(&owner), &user.did)?;

    series_service::remove_series_article(&state.pool, &id, &input.article_uri)
        .await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub(crate) struct AddSeriesPrereqInput {
    article_uri: String,
    prereq_article_uri: String,
}

pub async fn add_series_prereq(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<AddSeriesPrereqInput>,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &id).await?;
    require_owner(Some(&owner), &user.did)?;

    series_service::add_series_prereq(
        &state.pool,
        &id,
        &input.article_uri,
        &input.prereq_article_uri,
    )
    .await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub(crate) struct RemoveSeriesPrereqInput {
    article_uri: String,
    prereq_article_uri: String,
}

pub async fn remove_series_prereq(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<RemoveSeriesPrereqInput>,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &id).await?;
    require_owner(Some(&owner), &user.did)?;

    series_service::remove_series_prereq(
        &state.pool,
        &id,
        &input.article_uri,
        &input.prereq_article_uri,
    )
    .await?;
    Ok(StatusCode::OK)
}

// --- All series articles (for homepage dedup) ---

#[derive(serde::Deserialize)]
pub struct BulkLimitQuery {
    pub limit: Option<i64>,
}

pub async fn all_series_articles(
    State(state): State<AppState>,
    Query(q): Query<BulkLimitQuery>,
) -> ApiResult<Json<Vec<series_service::SeriesArticleMemberRow>>> {
    let limit = q.limit.unwrap_or(10_000).clamp(1, 50_000);
    let rows = series_service::all_series_articles(&state.pool, limit).await?;
    Ok(Json(rows))
}

// --- Series context for article navigation (DAG-based) ---

pub async fn get_series_context(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<series_service::SeriesContextItem>>> {
    let context = series_service::get_series_context(&state.pool, &uri).await?;
    Ok(Json(context))
}

// --- Series tree (full hierarchy) ---

pub async fn get_series_tree(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<series_service::SeriesTreeNode>> {
    let tree = series_service::get_series_tree(&state.pool, &id).await?;
    Ok(Json(tree))
}

// --- Reorder articles within a series ---

#[derive(serde::Deserialize)]
pub(crate) struct ReorderArticlesInput {
    article_uris: Vec<String>,
}

pub async fn reorder_articles(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ReorderArticlesInput>,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &id).await?;
    require_owner(Some(&owner), &user.did)?;

    series_service::reorder_series_articles(&state.pool, &id, &input.article_uris)
        .await?;
    Ok(StatusCode::OK)
}

// --- Series resource upload ---

pub async fn upload_resource(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    mut multipart: Multipart,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &id).await?;
    require_owner(Some(&owner), &user.did)?;

    let pijul_node_id: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    let Some(node_id) = pijul_node_id else {
        return Err(AppError(fx_core::Error::BadRequest(
            "Series does not have a pijul repo. Only root series support resources.".into(),
        )));
    };

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError(fx_core::Error::BadRequest(format!("multipart error: {e}")))
    })? {
        let filename = field.file_name().unwrap_or("unnamed").to_string();
        // Validate filename: only allow safe names
        if filename.contains('/') || filename.contains("..") || filename.starts_with('.') {
            return Err(AppError(fx_core::Error::BadRequest(
                format!("Invalid filename: {filename}"),
            )));
        }
        let data = field.bytes().await.map_err(|e| {
            AppError(fx_core::Error::BadRequest(format!("read error: {e}")))
        })?;

        state.pijul.write_resource(&node_id, &filename, &data)
            .map_err(|e| AppError(fx_core::Error::Internal(format!("write resource: {e}"))))?;
    }

    // Record the change
    if let Err(e) = state.pijul.record(&node_id, "Upload shared resource", Some(&user.did)) {
        tracing::warn!("pijul record failed for series resource: {e}");
    }

    Ok(StatusCode::CREATED)
}

#[derive(serde::Serialize)]
pub struct ResourceInfo {
    pub filename: String,
    pub size: u64,
}

pub async fn list_resources(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ResourceInfo>>> {
    let pijul_node_id: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    let Some(node_id) = pijul_node_id else {
        return Ok(Json(vec![]));
    };

    let repo_path = state.pijul.series_repo_path(&node_id);
    let mut resources = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&repo_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Skip pijul internals and ignore files
                if name.starts_with('.') || name.ends_with(".html") {
                    continue;
                }
                if let Ok(meta) = entry.metadata() {
                    resources.push(ResourceInfo {
                        filename: name,
                        size: meta.len(),
                    });
                }
            }
        }
    }

    Ok(Json(resources))
}

// --- File tree: list, read, write, delete ---

#[derive(serde::Serialize)]
pub struct SeriesFileInfo {
    pub path: String,  // relative to repo root, e.g. "chapters/01.md"
    pub size: u64,
}

/// List all user-editable files in the series repo (chapters/* + root non-ignored files).
pub async fn list_series_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<SeriesFileInfo>>> {
    let pijul_node_id: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    let Some(node_id) = pijul_node_id else {
        return Ok(Json(vec![]));
    };

    let repo_path = state.pijul.series_repo_path(&node_id);
    let mut files = Vec::new();

    // Root-level editable files (skip hidden, cache, HTML)
    if let Ok(entries) = std::fs::read_dir(&repo_path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_file() { continue; }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name.ends_with(".html") { continue; }
            if let Ok(meta) = entry.metadata() {
                files.push(SeriesFileInfo { path: name, size: meta.len() });
            }
        }
    }

    // chapters/ directory
    let chapters_dir = repo_path.join("chapters");
    if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
        let mut chapter_files: Vec<_> = entries.flatten().filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            e.path().is_file() && !n.starts_with('.') && !n.ends_with(".html")
        }).collect();
        chapter_files.sort_by_key(|e| e.file_name());
        for entry in chapter_files {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(meta) = entry.metadata() {
                files.push(SeriesFileInfo { path: format!("chapters/{name}"), size: meta.len() });
            }
        }
    }

    Ok(Json(files))
}

#[derive(serde::Deserialize)]
pub struct FilePathQuery {
    pub path: String,
}

/// Read a single file's text content.
pub async fn read_series_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<FilePathQuery>,
) -> ApiResult<String> {
    let node_id = series_node_id(&state.pool, &id).await?;
    let safe = safe_path(&q.path)?;
    let full = state.pijul.series_repo_path(&node_id).join(&safe);
    let content = tokio::fs::read_to_string(&full).await
        .map_err(|_| AppError(fx_core::Error::NotFound { entity: "file", id: q.path.clone() }))?;
    Ok(content)
}

#[derive(serde::Deserialize)]
pub struct WriteFileInput {
    pub path: String,
    pub content: String,
    pub message: Option<String>,
}

/// Write (create or overwrite) a file, then record in pijul.
pub async fn write_series_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<WriteFileInput>,
) -> ApiResult<StatusCode> {
    let node_id = series_node_id(&state.pool, &id).await?;
    require_series_owner(&state.pool, &id, &user.did).await?;

    let safe = safe_path(&input.path)?;
    let full = state.pijul.series_repo_path(&node_id).join(&safe);
    if let Some(parent) = full.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&full, &input.content).await?;

    let msg = input.message.unwrap_or_else(|| format!("Update {}", input.path));
    if let Err(e) = state.pijul.record_series(&node_id, &msg, Some(&user.did)) {
        tracing::warn!("pijul record failed for {node_id}: {e}");
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Delete a file from the series repo.
pub async fn delete_series_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Query(q): Query<FilePathQuery>,
) -> ApiResult<StatusCode> {
    let node_id = series_node_id(&state.pool, &id).await?;
    require_series_owner(&state.pool, &id, &user.did).await?;

    let safe = safe_path(&q.path)?;
    let full = state.pijul.series_repo_path(&node_id).join(&safe);
    tokio::fs::remove_file(&full).await
        .map_err(|_| AppError(fx_core::Error::NotFound { entity: "file", id: q.path.clone() }))?;

    if let Err(e) = state.pijul.record_series(&node_id, &format!("Delete {}", q.path), Some(&user.did)) {
        tracing::warn!("pijul record failed: {e}");
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---- helpers ----

async fn series_node_id(pool: &sqlx::PgPool, series_id: &str) -> ApiResult<String> {
    let node_id: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(series_id)
    .fetch_optional(pool)
    .await?
    .flatten();

    node_id.ok_or_else(|| AppError(fx_core::Error::BadRequest("Series has no pijul repo".into())))
}

async fn require_series_owner(pool: &sqlx::PgPool, series_id: &str, did: &str) -> ApiResult<()> {
    let owner = series_service::get_series_owner(pool, series_id).await?;
    require_owner(Some(&owner), did)
}

/// Validate and normalise a relative file path; reject traversal attempts.
fn safe_path(path: &str) -> ApiResult<std::path::PathBuf> {
    if path.contains("..") || path.starts_with('/') {
        return Err(AppError(fx_core::Error::BadRequest(format!("Invalid path: {path}"))));
    }
    let p = std::path::Path::new(path);
    // Only allow one level of subdirectory (chapters/<name> or <name>)
    let components: Vec<_> = p.components().collect();
    if components.len() > 2 {
        return Err(AppError(fx_core::Error::BadRequest("Path too deep".into())));
    }
    Ok(p.to_path_buf())
}

// --- Fork series ---

pub async fn fork_series(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
) -> ApiResult<(StatusCode, Json<series_service::SeriesRow>)> {
    // Get original series
    let original = series_service::get_series_detail(&state.pool, &id).await?;

    let fork_id = format!("s-{}", tid());
    let fork_node_id = format!("series_{fork_id}");

    // Fork pijul repo if original has one
    let original_pijul: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    if let Some(ref source_node) = original_pijul {
        state.pijul.fork_series(source_node, &fork_node_id)
            .map_err(|e| AppError(fx_core::Error::Internal(format!("fork series repo: {e}"))))?;
    }

    // Create forked series record
    let fork_title = format!("{} (fork)", original.series.title);
    let fork_pijul = if original_pijul.is_some() { Some(fork_node_id.as_str()) } else { None };
    let row = series_service::create_series(
        &state.pool,
        &fork_id,
        &fork_title,
        original.series.description.as_deref(),
        original.series.long_description.as_deref(),
        &[],
        &user.did,
        &original.series.lang,
        None,
        &original.series.category,
        fork_pijul,
    )
    .await?;

    // Fork only clones the pijul repo. User calls /compile to create articles.

    Ok((StatusCode::CREATED, Json(row)))
}

// ---- Series compile: heading extraction + article sync ----

#[derive(serde::Serialize)]
pub struct CompileResult {
    pub articles_created: usize,
    pub articles_updated: usize,
    pub total_headings: usize,
}

/// Compile a series: render all content, extract headings, create/update article slices.
pub async fn compile_series(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(series_id): Path<String>,
) -> ApiResult<Json<CompileResult>> {
    // Verify ownership
    let owner = series_service::get_series_owner(&state.pool, &series_id).await?;
    require_owner(Some(&owner), &user.did)?;

    // Load series
    let series = series_service::get_series_detail(&state.pool, &series_id).await?;
    let split_level = series.series.split_level as u32;

    let pijul_node_id: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&series_id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    let node_id = pijul_node_id.ok_or_else(|| {
        AppError(fx_core::Error::BadRequest("Series has no pijul repo".into()))
    })?;

    let series_repo = state.pijul.series_repo_path(&node_id);

    // Determine content format from chapter files
    let chapters_dir = series_repo.join("chapters");
    let has_typst = chapters_dir.exists() && std::fs::read_dir(&chapters_dir)
        .map(|entries| entries.flatten().any(|e| {
            e.file_name().to_string_lossy().ends_with(".typ")
        }))
        .unwrap_or(false);
    let has_markdown = chapters_dir.exists() && std::fs::read_dir(&chapters_dir)
        .map(|entries| entries.flatten().any(|e| {
            e.file_name().to_string_lossy().ends_with(".md")
        }))
        .unwrap_or(false);

    // Render to full HTML
    let repo = series_repo.clone();
    let full_html = if has_typst || series_repo.join("main.typ").exists() {
        tokio::task::spawn_blocking(move || {
            fx_render::render_series_full_html(&repo)
        }).await.map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))??
    } else if has_markdown {
        // Read all .md chapters
        let mut md_chapters = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
            let mut files: Vec<_> = entries.flatten().filter(|e| {
                e.file_name().to_string_lossy().ends_with(".md")
            }).collect();
            files.sort_by_key(|e| e.file_name());
            for entry in files {
                let content = tokio::fs::read_to_string(entry.path()).await?;
                let uri = entry.file_name().to_string_lossy().to_string();
                md_chapters.push((uri, content));
            }
        }
        fx_render::render_markdown_series(&md_chapters)
            .map_err(|e| AppError(fx_core::Error::Render(e.to_string())))?
    } else {
        return Err(AppError(fx_core::Error::BadRequest("No compilable content found".into())));
    };

    // Extract headings and split
    let headings = fx_render::heading_extract::extract_headings(&full_html);
    let slices = fx_render::heading_extract::split_at_level(&full_html, &headings, split_level);
    let heading_tree = fx_render::heading_extract::build_heading_tree(&headings);

    // Load existing heading-based articles
    let existing: Vec<(String, String)> = sqlx::query_as(
        "SELECT article_uri, heading_anchor FROM series_articles \
         WHERE series_id = $1 AND heading_anchor IS NOT NULL",
    )
    .bind(&series_id)
    .fetch_all(&state.pool)
    .await?;

    let existing_map: std::collections::HashMap<String, String> = existing
        .into_iter()
        .map(|(uri, anchor)| (anchor, uri))
        .collect();

    let mut created = 0usize;
    let mut updated = 0usize;

    // Ensure cache dir exists
    let cache_dir = series_repo.join("cache");
    let _ = tokio::fs::create_dir_all(&cache_dir).await;

    for (order, slice) in slices.iter().enumerate() {
        if let Some(existing_uri) = existing_map.get(&slice.heading_anchor) {
            // Update existing article
            sqlx::query(
                "UPDATE series_articles SET heading_title = $1, order_index = $2 \
                 WHERE series_id = $3 AND article_uri = $4",
            )
            .bind(&slice.heading_title)
            .bind(order as i32)
            .bind(&series_id)
            .bind(existing_uri)
            .execute(&state.pool)
            .await?;

            // Update article title
            sqlx::query("UPDATE articles SET title = $1 WHERE at_uri = $2")
                .bind(&slice.heading_title)
                .bind(existing_uri)
                .execute(&state.pool)
                .await?;

            // Write cache
            let _ = tokio::fs::write(
                cache_dir.join(format!("{}.html", slice.heading_anchor)),
                &slice.html,
            ).await;

            updated += 1;
        } else {
            // Create new article for this heading
            let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::ARTICLE, tid());

            // Insert article record (minimal — content lives in series cache)
            let hash = fx_core::util::content_hash(&slice.html);
            let input = CreateArticle {
                title: slice.heading_title.clone(),
                description: None,
                content: String::new(), // content is in cache
                content_format: ContentFormat::Html,
                lang: Some(series.series.lang.clone()),
                license: None,
                translation_of: None,
                restricted: None,
                category: Some(series.series.category.clone()),
                book_id: None,
                edition_id: None,
                tags: vec![],
                prereqs: vec![],
                series_id: Some(series_id.clone()),
                invites: vec![],
            };
            article_service::create_article(
                &state.pool, &user.did, &at_uri, &input, &hash, None,
                "public", ContentKind::Article, None,
            ).await?;

            // Insert series_articles with heading info
            sqlx::query(
                "INSERT INTO series_articles (series_id, article_uri, order_index, heading_title, heading_anchor) \
                 VALUES ($1, $2, $3, $4, $5) ON CONFLICT (series_id, article_uri) DO UPDATE \
                 SET heading_title = $4, heading_anchor = $5, order_index = $3",
            )
            .bind(&series_id)
            .bind(&at_uri)
            .bind(order as i32)
            .bind(&slice.heading_title)
            .bind(&slice.heading_anchor)
            .execute(&state.pool)
            .await?;

            // Write cache
            let _ = tokio::fs::write(
                cache_dir.join(format!("{}.html", slice.heading_anchor)),
                &slice.html,
            ).await;

            created += 1;
        }
    }

    // Update series_headings table (full TOC)
    sqlx::query("DELETE FROM series_headings WHERE series_id = $1")
        .bind(&series_id)
        .execute(&state.pool)
        .await?;

    fn insert_heading_tree(
        nodes: &[fx_render::heading_extract::HeadingNode],
        series_id: &str,
        parent_id: Option<i32>,
        order_start: &mut i32,
        inserts: &mut Vec<(String, i32, String, String, Option<i32>, i32)>,
    ) {
        for node in nodes {
            let order = *order_start;
            *order_start += 1;
            inserts.push((
                series_id.to_string(),
                node.level as i32,
                node.title.clone(),
                node.anchor.clone(),
                parent_id,
                order,
            ));
            // Children will reference parent, but we don't know the ID yet
            // For simplicity, skip parent_heading_id linkage for now
            insert_heading_tree(&node.children, series_id, None, order_start, inserts);
        }
    }

    let mut inserts = Vec::new();
    let mut order = 0i32;
    insert_heading_tree(&heading_tree, &series_id, None, &mut order, &mut inserts);

    for (sid, level, title, anchor, _parent, order_idx) in &inserts {
        sqlx::query(
            "INSERT INTO series_headings (series_id, level, title, anchor, order_index) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(sid)
        .bind(level)
        .bind(title)
        .bind(anchor)
        .bind(order_idx)
        .execute(&state.pool)
        .await?;
    }

    // Touch series cache marker
    let _ = tokio::fs::write(series_repo.join("cache").join("series.cache"), "").await;

    Ok(Json(CompileResult {
        articles_created: created,
        articles_updated: updated,
        total_headings: headings.len(),
    }))
}

// ---- Get headings (TOC) ----

pub async fn get_headings(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
) -> ApiResult<Json<Vec<series_service::SeriesHeadingRow>>> {
    let rows = sqlx::query_as::<_, series_service::SeriesHeadingRow>(
        "SELECT id, series_id, level, title, anchor, article_uri, parent_heading_id, order_index \
         FROM series_headings WHERE series_id = $1 ORDER BY order_index",
    )
    .bind(&series_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// ---- Update split level ----

#[derive(serde::Deserialize)]
pub struct UpdateSplitLevel {
    split_level: i32,
}

pub async fn update_split_level(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(series_id): Path<String>,
    Json(input): Json<UpdateSplitLevel>,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &series_id).await?;
    require_owner(Some(&owner), &user.did)?;

    if !(1..=6).contains(&input.split_level) {
        return Err(AppError(fx_core::Error::BadRequest("split_level must be 1-6".into())));
    }

    sqlx::query("UPDATE series SET split_level = $1 WHERE id = $2")
        .bind(input.split_level)
        .bind(&series_id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Collaboration endpoints ---

pub async fn list_collaborators(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<collaboration_service::Collaborator>>> {
    let collabs = collaboration_service::list_collaborators(&state.pool, &id).await?;
    Ok(Json(collabs))
}

#[derive(serde::Deserialize)]
pub struct InviteInput {
    pub user_did: String,
    pub role: Option<String>,
}

pub async fn invite_collaborator(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<InviteInput>,
) -> ApiResult<(StatusCode, Json<collaboration_service::Collaborator>)> {
    let owner = series_service::get_series_owner(&state.pool, &id).await?;
    require_owner(Some(&owner), &user.did)?;

    let role = input.role.as_deref().unwrap_or("editor");
    let short_did = input.user_did.chars().rev().take(8).collect::<String>().chars().rev().collect::<String>();
    let channel_name = format!("collab_{short_did}");

    let node_id: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    if let Some(ref node) = node_id {
        state.pijul.create_channel(node, &channel_name, None)
            .map_err(|e| AppError(fx_core::Error::Internal(format!("create channel: {e}"))))?;
    }

    let collab = collaboration_service::add_collaborator(
        &state.pool, &id, &input.user_did, role, &channel_name, &user.did,
    ).await?;

    Ok((StatusCode::CREATED, Json(collab)))
}

pub async fn remove_collaborator(
    State(state): State<AppState>,
    Path((id, did)): Path<(String, String)>,
    WriteAuth(user): WriteAuth,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &id).await?;
    require_owner(Some(&owner), &user.did)?;

    let collab = collaboration_service::get_collaborator(&state.pool, &id, &did).await?;
    let removed = collaboration_service::remove_collaborator(&state.pool, &id, &did).await?;
    if !removed {
        return Err(AppError(fx_core::Error::NotFound { entity: "collaborator", id: did.clone() }));
    }

    if let Some(c) = collab {
        let node_id: Option<String> = sqlx::query_scalar(
            "SELECT pijul_node_id FROM series WHERE id = $1",
        )
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?
        .flatten();

        if let Some(ref node) = node_id {
            let _ = state.pijul.delete_channel(node, &c.channel_name);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_channels(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<String>>> {
    let node_id: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    let channels = match node_id {
        Some(ref node) => state.pijul.list_channels(node)
            .map_err(|e| AppError(fx_core::Error::Internal(format!("list channels: {e}"))))?,
        None => vec!["main".to_string()],
    };

    Ok(Json(channels))
}

#[derive(serde::Deserialize)]
pub struct ChannelFileQuery {
    pub path: String,
}

pub async fn read_channel_file(
    State(state): State<AppState>,
    Path((id, channel)): Path<(String, String)>,
    Query(q): Query<ChannelFileQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let node_id: String = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten()
    .ok_or(AppError(fx_core::Error::NotFound { entity: "series", id: id.clone() }))?;

    let content = state.pijul.read_file_from_channel(&node_id, &channel, &q.path)
        .map_err(|e| AppError(fx_core::Error::Internal(format!("read channel file: {e}"))))?;

    let text = String::from_utf8_lossy(&content).into_owned();
    Ok(Json(serde_json::json!({ "content": text })))
}

#[derive(serde::Deserialize)]
pub struct WriteChannelFileInput {
    pub path: String,
    pub content: String,
    pub message: Option<String>,
}

pub async fn write_channel_file(
    State(state): State<AppState>,
    Path((id, channel)): Path<(String, String)>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<WriteChannelFileInput>,
) -> ApiResult<Json<serde_json::Value>> {
    let collab = collaboration_service::get_collaborator(&state.pool, &id, &user.did).await?;
    let allowed = match &collab {
        Some(c) => c.channel_name == channel || c.role == "owner",
        None => false,
    };
    if !allowed {
        return Err(AppError(fx_core::Error::Forbidden { action: "write to this channel" }));
    }

    let node_id: String = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten()
    .ok_or(AppError(fx_core::Error::NotFound { entity: "series", id: id.clone() }))?;

    let msg = input.message.as_deref().unwrap_or("update file");
    let result = state.pijul.write_and_record_on_channel(
        &node_id, &channel, &input.path, input.content.as_bytes(), msg, Some(&user.did),
    ).map_err(|e| AppError(fx_core::Error::Internal(format!("write channel file: {e}"))))?;

    let (hash, merkle) = result.unwrap_or_default();
    Ok(Json(serde_json::json!({ "change_hash": hash, "merkle": merkle })))
}

pub async fn channel_log(
    State(state): State<AppState>,
    Path((id, channel)): Path<(String, String)>,
) -> ApiResult<Json<Vec<String>>> {
    let node_id: String = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten()
    .ok_or(AppError(fx_core::Error::NotFound { entity: "series", id: id.clone() }))?;

    let log = state.pijul.log_channel(&node_id, &channel)
        .map_err(|e| AppError(fx_core::Error::Internal(format!("channel log: {e}"))))?;

    Ok(Json(log))
}

#[derive(serde::Deserialize)]
pub struct ApplyChangeInput {
    pub source_channel: String,
    pub change_hash: String,
}

pub async fn apply_channel_change(
    State(state): State<AppState>,
    Path((id, channel)): Path<(String, String)>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ApplyChangeInput>,
) -> ApiResult<StatusCode> {
    let collab = collaboration_service::get_collaborator(&state.pool, &id, &user.did).await?;
    let allowed = match &collab {
        Some(c) => c.channel_name == channel || c.role == "owner",
        None => false,
    };
    if !allowed {
        return Err(AppError(fx_core::Error::Forbidden { action: "write to this channel" }));
    }

    let node_id: String = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten()
    .ok_or(AppError(fx_core::Error::NotFound { entity: "series", id: id.clone() }))?;

    state.pijul.apply_change_to_channel(&node_id, &input.change_hash, &channel)
        .map_err(|e| AppError(fx_core::Error::Internal(format!("apply change: {e}"))))?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct ChannelDiffQuery {
    pub a: String,
    pub b: String,
}

pub async fn channel_diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ChannelDiffQuery>,
) -> ApiResult<Json<fx_pijul::ChannelDiffResult>> {
    let node_id: String = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .flatten()
    .ok_or(AppError(fx_core::Error::NotFound { entity: "series", id: id.clone() }))?;

    let diff = state.pijul.diff_channels(&node_id, &q.a, &q.b)
        .map_err(|e| AppError(fx_core::Error::Internal(format!("channel diff: {e}"))))?;

    Ok(Json(diff))
}
