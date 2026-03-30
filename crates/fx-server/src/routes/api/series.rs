use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
};
use fx_core::services::series_service;
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
    parent_id: Option<String>,
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

    // If parent_id given, verify the user owns the parent series
    if let Some(ref pid) = input.parent_id {
        let owner = series_service::get_series_owner(&state.pool, pid).await?;
        require_owner(Some(&owner), &user.did)?;
    }

    let lang = input.lang.as_deref().unwrap_or("zh");
    let translation_group = if let Some(ref source_id) = input.translation_of {
        Some(series_service::resolve_series_translation_group(&state.pool, source_id).await?)
    } else {
        None
    };

    let category = input.category.as_deref().unwrap_or("general");
    // Initialize pijul repo for the series (only for root series, not sub-series)
    let pijul_node_id = if input.parent_id.is_none() {
        let node_id = format!("series_{id}");
        if let Err(e) = state.pijul.init_series_repo(&node_id) {
            tracing::warn!("failed to init series pijul repo: {e}");
        }
        Some(node_id)
    } else {
        None
    };

    let row = series_service::create_series(
        &state.pool,
        &id,
        &input.title,
        input.description.as_deref(),
        input.long_description.as_deref(),
        &topics,
        input.parent_id.as_deref(),
        &user.did,
        lang,
        translation_group,
        category,
    )
    .await?;

    // Store pijul_node_id
    if let Some(ref node_id) = pijul_node_id {
        sqlx::query("UPDATE series SET pijul_node_id = $1 WHERE id = $2")
            .bind(node_id)
            .bind(&id)
            .execute(&state.pool)
            .await?;
    }

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

// --- Reorder child series ---

#[derive(serde::Deserialize)]
pub(crate) struct ReorderChildrenInput {
    child_ids: Vec<String>,
}

pub async fn reorder_children(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ReorderChildrenInput>,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &id).await?;
    require_owner(Some(&owner), &user.did)?;

    series_service::reorder_children(&state.pool, &id, &input.child_ids).await?;
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
    if let Err(e) = state.pijul.record(&node_id, "Upload shared resource") {
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
    let row = series_service::create_series(
        &state.pool,
        &fork_id,
        &fork_title,
        original.series.description.as_deref(),
        original.series.long_description.as_deref(),
        &[],
        None,
        &user.did,
        &original.series.lang,
        None,
        &original.series.category,
    )
    .await?;

    // Store pijul_node_id
    if original_pijul.is_some() {
        sqlx::query("UPDATE series SET pijul_node_id = $1 WHERE id = $2")
            .bind(&fork_node_id)
            .bind(&fork_id)
            .execute(&state.pool)
            .await?;
    }

    // TODO: Create forked articles for each chapter and add to new series.
    // For now, the pijul repo is forked but articles need to be created manually.

    Ok((StatusCode::CREATED, Json(row)))
}
