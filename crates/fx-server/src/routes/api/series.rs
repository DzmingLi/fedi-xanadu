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
    summary: Option<String>,
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

    let desc_html = match input.summary.as_deref() {
        Some(desc) if !desc.is_empty() => {
            let repo_path = pijul_node_id.as_deref()
                .map(|n| state.pijul.series_repo_path(n))
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            crate::summary::render_summary_inline("markdown", desc, &repo_path)
                .unwrap_or_default()
        }
        _ => String::new(),
    };

    let row = series_service::create_series(
        &state.pool,
        &id,
        &input.title,
        input.summary.as_deref(),
        &desc_html,
        input.long_description.as_deref(),
        &topics,
        &user.did,
        lang,
        translation_group,
        category,
        pijul_node_id.as_deref(),
    )
    .await?;

    // Write series meta.yaml
    if let Some(ref node) = pijul_node_id {
        let meta = fx_core::meta::SeriesMeta {
            title: input.title.clone(),
            description: input.summary.clone(),
            long_description: input.long_description.clone(),
            lang: Some(lang.to_string()),
            category: Some(category.to_string()),
            topics: topics.clone(),
            split_level: None,
            chapters: Vec::new(),
        };
        let repo_path = state.pijul.series_repo_path(node);
        if let Err(e) = fx_core::meta::write_series_meta(&repo_path, &meta) {
            tracing::warn!("failed to write series meta.yaml: {e}");
        }
        let _ = state.pijul_record_series(node.clone(), "Add metadata".into(), Some(user.did.clone())).await;
    }

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

    // Linking an existing standalone article to a series: its content lives
    // in its own per-article pijul repo, so repo_path is None.
    series_service::add_series_article(&state.pool, &id, &input.article_uri, None).await?;
    Ok(StatusCode::NO_CONTENT)
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
    Ok(StatusCode::NO_CONTENT)
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
    Ok(StatusCode::NO_CONTENT)
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
    Ok(StatusCode::NO_CONTENT)
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
    Ok(StatusCode::NO_CONTENT)
}

// --- Serve files from series repo ---

/// Serve any file from a series repo by path.
/// URL: GET /api/series/{id}/res/{*path}
/// e.g. /api/series/s-xxx/res/ch01-intro/images/logo.png
pub async fn serve_file(
    State(state): State<AppState>,
    Path((id, file_path)): Path<(String, String)>,
) -> axum::response::Response<axum::body::Body> {
    use axum::http::{header, StatusCode};
    use axum::response::Response;
    use axum::body::Body;

    // Look up pijul_node_id for this series
    let node_id: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1 AND pijul_node_id IS NOT NULL"
    ).bind(&id).fetch_optional(&state.pool).await.ok().flatten();

    let Some(node_id) = node_id else {
        return Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap();
    };

    let series_repo = state.pijul.series_repo_path(&node_id);

    // Sanitize path: no ".." traversal
    let clean_path: String = file_path.split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect::<Vec<_>>()
        .join("/");

    let full_path = series_repo.join(&clean_path);

    // Ensure it's within the repo
    if !full_path.starts_with(&series_repo) {
        return Response::builder().status(StatusCode::FORBIDDEN).body(Body::empty()).unwrap();
    }

    match tokio::fs::read(&full_path).await {
        Ok(data) => {
            let content_type = match full_path.extension().and_then(|e| e.to_str()) {
                Some("png") => "image/png",
                Some("jpg" | "jpeg") => "image/jpeg",
                Some("gif") => "image/gif",
                Some("svg") => "image/svg+xml",
                Some("webp") => "image/webp",
                Some("pdf") => "application/pdf",
                Some("css") => "text/css",
                Some("js") => "application/javascript",
                _ => "application/octet-stream",
            };
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=86400")
                .body(Body::from(data))
                .unwrap()
        }
        Err(_) => Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap(),
    }
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
    if let Err(e) = state.pijul_record(node_id.clone(), "Upload shared resource".into(), Some(user.did.clone())).await {
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

// File/channel operations now handled by pijul_knot::pad_router

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
        original.series.summary.as_deref(),
        &original.series.summary_html,
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

    // Sync series meta.yaml → DB if it exists
    if let Some(ref node) = pijul_node_id {
        let repo_path = state.pijul.series_repo_path(node);
        if let Some(meta) = fx_core::meta::read_series_meta(&repo_path) {
            if !meta.title.is_empty() {
                let _ = sqlx::query("UPDATE series SET title = $1 WHERE id = $2")
                    .bind(&meta.title).bind(&series_id).execute(&state.pool).await;
            }
            if let Some(ref desc) = meta.description {
                let _ = sqlx::query("UPDATE series SET summary = $1 WHERE id = $2")
                    .bind(desc).bind(&series_id).execute(&state.pool).await;
            }
            if let Some(ref long_desc) = meta.long_description {
                let _ = sqlx::query("UPDATE series SET long_description = $1 WHERE id = $2")
                    .bind(long_desc).bind(&series_id).execute(&state.pool).await;
            }
            if let Some(level) = meta.split_level {
                if (1..=6).contains(&level) {
                    let _ = sqlx::query("UPDATE series SET split_level = $1 WHERE id = $2")
                        .bind(level as i32).bind(&series_id).execute(&state.pool).await;
                }
            }
        }
    }

    let node_id = pijul_node_id.ok_or_else(|| {
        AppError(fx_core::Error::BadRequest("Series has no pijul repo".into()))
    })?;

    let series_repo = state.pijul.series_repo_path(&node_id);

    // Determine content format from repo root files
    let has_typst = std::fs::read_dir(&series_repo)
        .map(|entries| entries.flatten().any(|e| {
            e.file_name().to_string_lossy().ends_with(".typ")
        }))
        .unwrap_or(false);
    let has_markdown = std::fs::read_dir(&series_repo)
        .map(|entries| entries.flatten().any(|e| {
            e.file_name().to_string_lossy().ends_with(".md")
        }))
        .unwrap_or(false);

    // Render to full HTML
    let repo = series_repo.clone();
    let full_html = if has_typst || series_repo.join("main.typ").exists() {
        tokio::task::spawn_blocking(move || {
            let config = fx_renderer::fx_render_config();
            fx_renderer::typst_render::render_series_full_html_with_config(&repo, &config)
        }).await.map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))??
    } else if has_markdown {
        // Read .md chapters in meta.yaml order (or sorted fallback)
        let md_order = fx_renderer::read_chapter_order(&series_repo, ".md");
        let mut md_chapters = Vec::new();
        for name in &md_order {
            let path = series_repo.join(name);
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                md_chapters.push((name.clone(), content));
            }
        }
        fx_renderer::render_markdown_series(&md_chapters)
            .map_err(|e| AppError(fx_core::Error::Render(e.to_string())))?
    } else {
        return Err(AppError(fx_core::Error::BadRequest("No compilable content found".into())));
    };

    // Extract headings and split
    let headings = fx_renderer::heading_extract::extract_headings(&full_html);
    let slices = fx_renderer::heading_extract::split_at_level(&full_html, &headings, split_level);
    let heading_tree = fx_renderer::heading_extract::build_heading_tree(&headings);

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
                summary: None,
                content: String::new(), // content is in cache
                content_format: ContentFormat::Html,
                lang: Some(series.series.lang.clone()),
                license: None,
                translation_of: None,
                restricted: None,
                category: Some(series.series.category.clone()),
                tags: vec![],
                prereqs: vec![],
                series_id: Some(series_id.clone()),
                metadata: None,
                authors: vec![],
                invites: vec![],
            };
            article_service::create_article(
                &state.pool, &user.did, &at_uri, &input, &hash, None,
                "public", ContentKind::Article, None,
                "", "",
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
        nodes: &[fx_renderer::heading_extract::HeadingNode],
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

    // Typst-only: query <nbt-chapter> and <nbt-summary> labels, pair them
    // with the split slices in document order, and sync the teaches /
    // prereqs / summary back to DB. Failures here are non-fatal — the
    // compile itself already succeeded.
    if has_typst || series_repo.join("main.typ").exists() {
        let repo = series_repo.clone();
        let metadata = tokio::task::spawn_blocking(move || {
            let config = fx_renderer::fx_render_config();
            fx_renderer::extract_series_metadata(&repo, &config)
        }).await.ok().and_then(|r| r.ok());
        if let Some(meta) = metadata {
            sync_typst_chapter_metadata(&state, &series_id, &slices, &meta).await;
        }
    }

    Ok(Json(CompileResult {
        articles_created: created,
        articles_updated: updated,
        total_headings: headings.len(),
    }))
}

/// Pair the ordered `<nbt-chapter>` / `<nbt-summary>` extractor results
/// with the split-output slices and write the teaches / prereqs / summary
/// fields back to DB.
///
/// Convention: metadata entries appear in the Typst source in the same order
/// as the headings they annotate, so `metadata[i]` belongs to `slice[i]` for
/// whichever slices have at least one metadata entry preceding them.
async fn sync_typst_chapter_metadata(
    state: &AppState,
    series_id: &str,
    slices: &[fx_renderer::heading_extract::HtmlSlice],
    meta: &fx_renderer::SeriesMetadata,
) {
    // Load the chapter article URIs in slice order.
    let chapter_uris: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT heading_anchor, article_uri FROM series_articles \
         WHERE series_id = $1 AND heading_anchor IS NOT NULL",
    )
    .bind(series_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();
    let anchor_to_uri: std::collections::HashMap<_, _> = chapter_uris.into_iter().collect();

    for (i, slice) in slices.iter().enumerate() {
        let Some(uri) = anchor_to_uri.get(&slice.heading_anchor) else { continue };

        if let Some(value) = meta.chapter_metadata.get(i) {
            // teaches: array of tag ids
            if let Some(arr) = value.get("teaches").and_then(|v| v.as_array()) {
                let _ = sqlx::query("DELETE FROM content_teaches WHERE content_uri = $1")
                    .bind(uri).execute(&state.pool).await;
                for tag in arr.iter().filter_map(|v| v.as_str()) {
                    let _ = sqlx::query(
                        "INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                    ).bind(uri).bind(tag).execute(&state.pool).await;
                }
            }
            // prereqs: array of (tag, type) tuples — also accepts { tag, type } dict
            if let Some(arr) = value.get("prereqs").and_then(|v| v.as_array()) {
                let _ = sqlx::query("DELETE FROM content_prereqs WHERE content_uri = $1")
                    .bind(uri).execute(&state.pool).await;
                for entry in arr {
                    let (tag, kind) = match entry {
                        serde_json::Value::String(s) => (Some(s.as_str()), "required"),
                        serde_json::Value::Array(t) => (
                            t.first().and_then(|v| v.as_str()),
                            t.get(1).and_then(|v| v.as_str()).unwrap_or("required"),
                        ),
                        serde_json::Value::Object(m) => (
                            m.get("tag").and_then(|v| v.as_str()),
                            m.get("type").and_then(|v| v.as_str()).unwrap_or("required"),
                        ),
                        _ => (None, "required"),
                    };
                    if let Some(tag) = tag {
                        let _ = sqlx::query(
                            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                        ).bind(uri).bind(tag).bind(kind).execute(&state.pool).await;
                    }
                }
            }
        }

        if let Some(summary) = meta.summaries.get(i).filter(|s| !s.is_empty()) {
            let html = crate::summary::render_summary_inline("typst", summary, std::path::Path::new("")).unwrap_or_default();
            let _ = article_service::update_article_summary(&state.pool, uri, summary, &html).await;
        }
    }
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

