use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fx_core::models::*;
use fx_core::services::{article_service, collaboration_service, series_service};
use fx_core::validation;

use crate::error::{AppError, ApiResult, require_owner};
use crate::state::AppState;
use crate::auth::{WriteAuth, pds_create_record};
use fx_core::util::{tid, now_rfc3339};
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
        #[allow(deprecated)]
        Some(series_service::resolve_series_translation_group(&state.pool, source_id).await?)
    } else {
        None
    };

    let category = input.category.as_deref().unwrap_or("general");

    let desc_html = match input.summary.as_deref() {
        Some(desc) if !desc.is_empty() => {
            crate::summary::render_summary_inline("markdown", desc, &state.blob_cache_path)
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
    )
    .await?;

    // Register creator as owner collaborator
    let _ = collaboration_service::register_owner(&state.pool, &id, &user.did).await;

    // Stamp series-level metadata into a `meta.yml` at the bundle root, then
    // upload it as the first file in the PDS record so the bundle is
    // self-describing. Per the unified `at.nightbo.work` lexicon, the record
    // carries only `files[]` + `createdAt`; chapter-level files are merged
    // in by `merge_chapter_into_series_record` on each chapter publish.
    let series_repo_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::WORK, &id);
    let mut files: Vec<serde_json::Value> = Vec::new();
    let meta_yaml = build_series_meta_yaml(
        &input.title, input.summary.as_deref(), lang, category, &topics,
    );
    let node_id = fx_core::util::uri_to_node_id(&series_repo_uri);
    let cache = state.blob_cache_path.join(&node_id);
    if tokio::fs::create_dir_all(&cache).await.is_ok() {
        let _ = tokio::fs::write(cache.join("meta.yml"), meta_yaml.as_bytes()).await;
    }
    let blob = crate::auth::upload_or_local_blob(
        &state, &user.token, &user.did, meta_yaml.as_bytes().to_vec(), "application/yaml",
    ).await;
    let cid = blob.get("ref").and_then(|r| r.get("$link"))
        .and_then(|c| c.as_str()).unwrap_or_default().to_string();
    files.push(serde_json::json!({
        "path": "meta.yml",
        "blob": blob,
        "mime": "application/yaml",
    }));
    let _ = cid;
    let record = serde_json::json!({
        "$type":     fx_atproto::lexicon::WORK,
        "files":     files,
        "createdAt": now_rfc3339(),
    });
    pds_create_record(&state, &user.token, fx_atproto::lexicon::WORK, record, Some(id.clone()), "create series").await;

    Ok((StatusCode::CREATED, Json(row)))
}

fn build_series_meta_yaml(
    title: &str,
    description: Option<&str>,
    lang: &str,
    category: &str,
    topics: &[String],
) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    writeln!(out, "title: {}", yaml_str(title)).ok();
    if let Some(d) = description.filter(|s| !s.is_empty()) {
        writeln!(out, "description: {}", yaml_str(d)).ok();
    }
    if !lang.is_empty()     { writeln!(out, "lang: {}", yaml_str(lang)).ok(); }
    if !category.is_empty() { writeln!(out, "category: {}", yaml_str(category)).ok(); }
    if !topics.is_empty() {
        writeln!(out, "topics:").ok();
        for t in topics { writeln!(out, "  - {}", yaml_str(t)).ok(); }
    }
    out
}

fn yaml_str(s: &str) -> String {
    // Always-quoted YAML scalar — sidesteps colon/leading-dash/etc edge cases.
    format!("\"{}\"", s.replace('\\', r"\\").replace('"', r#"\""#))
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

// Series-level shared resource endpoint. Under the unified `at.nightbo.work`
// model the series record's `files[]` covers both chapter sources and
// cross-chapter assets (figures, bibliographies, etc). On disk those land in
// `{blob_cache}/{uri_to_node_id(series_repo_uri)}/`, so serving a series
// resource is just static file serving from that directory.
//
// `upload_resource` and `list_resources` below stay no-op until we expose
// the bundle's files[] as a separate listing API; today the chapter publish
// flow is what materializes shared assets into the series cache directory.

pub async fn serve_file(
    State(state): State<AppState>,
    Path((id, file_path)): Path<(String, String)>,
) -> axum::response::Response<axum::body::Body> {
    use axum::http::{StatusCode, header};
    use axum::response::Response;
    use axum::body::Body;

    if file_path.is_empty() || file_path.contains("..") || file_path.starts_with('/') {
        return Response::builder().status(StatusCode::BAD_REQUEST).body(Body::empty()).unwrap();
    }
    let Ok(series_repo_uri) = series_service::series_repo_uri(&state.pool, &id).await else {
        return Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap();
    };
    let node_id = fx_core::util::uri_to_node_id(&series_repo_uri);
    let path = state.blob_cache_path.join(&node_id).join(&file_path);
    let Ok(bytes) = tokio::fs::read(&path).await else {
        return Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap();
    };
    let mime = match std::path::Path::new(&file_path).extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("pdf") => "application/pdf",
        Some("md") => "text/markdown",
        Some("typ") => "text/x-typst",
        Some("html" | "htm") => "text/html",
        Some("bib") => "application/x-bibtex",
        Some("json") => "application/json",
        Some("yaml" | "yml") => "application/yaml",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=300")
        .body(Body::from(bytes))
        .unwrap()
}

pub async fn upload_resource(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
    _auth: WriteAuth,
) -> ApiResult<StatusCode> {
    Err(AppError(fx_core::Error::BadRequest(
        "series-level shared resources are not supported in the blob storage model".into(),
    )))
}

#[derive(serde::Serialize)]
pub struct ResourceInfo {
    pub filename: String,
    pub size: u64,
}

pub async fn list_resources(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> ApiResult<Json<Vec<ResourceInfo>>> {
    Ok(Json(vec![]))
}

// ---- Series compile: heading extraction + article sync ----

#[derive(serde::Serialize)]
pub struct CompileResult {
    pub articles_created: usize,
    pub articles_updated: usize,
    pub total_headings: usize,
}

/// Compile a series: walks every chapter in `series_articles` order,
/// renders it with the series's shared blob_cache dir as working root (so
/// typst cross-chapter `@refs` resolve), caches rendered HTML under
/// `{series_cache}/cache/{anchor}.html`, and rebuilds `series_headings`.
///
/// Each heading row stores `(repo_uri, source_path)` pointing at the chapter
/// source. The wire format derives a `nightboat-chapter://` URI from those
/// columns server-side.
pub async fn compile_series(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(series_id): Path<String>,
) -> ApiResult<Json<CompileResult>> {
    let owner = series_service::get_series_owner(&state.pool, &series_id).await?;
    require_owner(Some(&owner), &user.did)?;
    compile_series_inner(&state, &series_id).await.map(Json)
}

/// Admin variant: same compile, no owner check. Used to backfill heading
/// data for series whose owner accounts are inactive (most legacy series
/// authored under `did:local:*` shells).
pub async fn admin_compile_series(
    State(state): State<AppState>,
    _admin: crate::auth::AdminAuth,
    Path(series_id): Path<String>,
) -> ApiResult<Json<CompileResult>> {
    compile_series_inner(&state, &series_id).await.map(Json)
}

async fn compile_series_inner(
    state: &AppState,
    series_id: &str,
) -> ApiResult<CompileResult> {
    let Some((series_repo_uri, chapters)) =
        series_service::get_series_chapters_for_render(&state.pool, series_id).await?
    else {
        return Err(AppError(fx_core::Error::NotFound { entity: "series", id: series_id.to_string() }));
    };

    if chapters.is_empty() {
        return Err(AppError(fx_core::Error::BadRequest("series has no chapters".into())));
    }

    // Series's shared on-disk root: cross-chapter refs + shared bibliography
    // resolve against this dir, not a per-chapter node dir.
    let series_node_id = fx_core::util::uri_to_node_id(&series_repo_uri);
    let series_root = state.blob_cache_path.join(&series_node_id);

    sqlx::query("DELETE FROM series_headings WHERE series_id = $1")
        .bind(series_id).execute(&state.pool).await?;

    let mut total_headings = 0usize;
    let mut order_index: i32 = 0;
    // Heading nesting is recovered from the level monotonicity of the
    // extracted h*'s via a parent stack: each new heading's parent is the
    // most-recent ancestor whose level is strictly smaller. Walking
    // chapters in order and resetting the stack per chapter keeps section
    // hierarchy local to each chapter (no cross-chapter parents).
    let mut stack: Vec<(i32, i32)> = Vec::new(); // (level, db_id)

    for ch in &chapters {
        stack.clear();
        let source_file = series_root.join(&ch.file_path);
        let Ok(src) = tokio::fs::read_to_string(&source_file).await else {
            continue;
        };

        let Ok(html) = fx_renderer::render_to_html(&ch.content_format, &src, &series_root) else {
            continue;
        };

        // Cache rendered HTML per-chapter so reading-page hits don't re-render.
        let anchor_stem: String = ch.heading_anchor.clone()
            .or_else(|| std::path::Path::new(&ch.source_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from))
            .unwrap_or_else(|| format!("chapter-{}", ch.order_index));
        let cache_file = series_root.join("cache").join(format!("{anchor_stem}.html"));
        if let Some(parent) = cache_file.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&cache_file, &html).await;

        // Globally-unique-within-series anchor prefix. anchor_stem alone
        // collides when several chapters share a stem (e.g. every chapter
        // is `chapterN/index.md`). source_path is unique per chapter by
        // construction, so use it for namespacing the heading anchors.
        let anchor_scope = ch.source_path.replace('/', ":");
        let headings = fx_renderer::heading_extract::extract_headings(&html);
        for h in headings {
            let level = h.level as i32;
            // Pop the stack until the top has strictly-smaller level —
            // that's this heading's nearest ancestor.
            while stack.last().is_some_and(|(l, _)| *l >= level) {
                stack.pop();
            }
            let parent_id = stack.last().map(|(_, id)| *id);
            // Prefix the chapter scope onto each anchor so two chapters
            // with the same heading text (e.g. "Introduction") don't
            // collide on the (series_id, anchor) UNIQUE constraint.
            let scoped_anchor = format!("{anchor_scope}/{}", h.anchor);
            let id: i32 = sqlx::query_scalar(
                "INSERT INTO series_headings \
                    (series_id, level, title, anchor, repo_uri, source_path, parent_heading_id, order_index) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
                 RETURNING id",
            )
            .bind(series_id)
            .bind(level)
            .bind(&h.title)
            .bind(&scoped_anchor)
            .bind(&series_repo_uri)
            .bind(&ch.source_path)
            .bind(parent_id)
            .bind(order_index)
            .fetch_one(&state.pool).await?;
            stack.push((level, id));
            order_index += 1;
            total_headings += 1;
        }
    }

    Ok(CompileResult {
        articles_created: 0,
        articles_updated: chapters.len(),
        total_headings,
    })
}

// ---- Get headings (TOC) ----

pub async fn get_headings(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
) -> ApiResult<Json<Vec<series_service::SeriesHeadingRow>>> {
    // The DB row carries (repo_uri, source_path); the wire format flattens
    // that into a `nightboat-chapter://` URI for leaf headings and `null` for
    // group headings.
    #[derive(sqlx::FromRow)]
    struct Row {
        id: i32,
        series_id: String,
        level: i32,
        title: String,
        anchor: String,
        source_path: Option<String>,
        parent_heading_id: Option<i32>,
        order_index: i32,
    }

    let raw = sqlx::query_as::<_, Row>(
        "SELECT id, series_id, level, title, anchor, source_path, \
                parent_heading_id, order_index \
         FROM series_headings WHERE series_id = $1 ORDER BY order_index",
    )
    .bind(&series_id)
    .fetch_all(&state.pool)
    .await?;

    let rows = raw.into_iter().map(|r| series_service::SeriesHeadingRow {
        article_uri: r.source_path.as_deref()
            .map(|sp| super::articles::build_chapter_uri(&r.series_id, sp)),
        id: r.id,
        series_id: r.series_id,
        level: r.level,
        title: r.title,
        anchor: r.anchor,
        parent_heading_id: r.parent_heading_id,
        order_index: r.order_index,
    }).collect();

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
    /// DID (did:plc:… / did:web:…) or atproto handle (e.g. alice.bsky.social).
    /// Handles are resolved server-side to a DID.
    pub identifier: String,
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

    let user_did = super::articles::resolve_identifier(&state, &input.identifier).await?;

    let role = input.role.as_deref().unwrap_or("editor");
    let short_did = user_did.chars().rev().take(8).collect::<String>().chars().rev().collect::<String>();
    let channel_name = format!("collab_{short_did}");

    let collab = collaboration_service::add_collaborator(
        &state.pool, &id, &user_did, role, &channel_name, &user.did,
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

    let removed = collaboration_service::remove_collaborator(&state.pool, &id, &did).await?;
    if !removed {
        return Err(AppError(fx_core::Error::NotFound { entity: "collaborator", id: did.clone() }));
    }

    Ok(StatusCode::NO_CONTENT)
}

