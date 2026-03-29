use axum::{
    Json,
    extract::{Path, Query, State},
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

#[utoipa::path(get, path = "/api/v1/series", responses((status = 200)))]
pub async fn list_series(
    State(state): State<AppState>,
    Query(q): Query<ListSeriesQuery>,
) -> ApiResult<Json<Vec<series_service::SeriesListRow>>> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let rows = series_service::list_series(&state.pool, limit).await?;
    Ok(Json(rows))
}

#[utoipa::path(post, path = "/api/v1/series", responses((status = 201)), security(("bearer" = [])))]
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

    Ok((StatusCode::CREATED, Json(row)))
}

#[utoipa::path(get, path = "/api/v1/series/{id}", params(("id" = String, Path)), responses((status = 200)))]
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

#[utoipa::path(post, path = "/api/v1/series/{id}/articles", params(("id" = String, Path)), responses((status = 200)), security(("bearer" = [])))]
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

#[utoipa::path(delete, path = "/api/v1/series/{id}/articles/remove", params(("id" = String, Path)), responses((status = 200)), security(("bearer" = [])))]
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

#[utoipa::path(post, path = "/api/v1/series/{id}/prereqs", params(("id" = String, Path)), responses((status = 200)), security(("bearer" = [])))]
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

#[utoipa::path(delete, path = "/api/v1/series/{id}/prereqs/remove", params(("id" = String, Path)), responses((status = 200)), security(("bearer" = [])))]
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

#[utoipa::path(get, path = "/api/v1/series/all-articles", responses((status = 200)))]
pub async fn all_series_articles(
    State(state): State<AppState>,
    Query(q): Query<BulkLimitQuery>,
) -> ApiResult<Json<Vec<series_service::SeriesArticleMemberRow>>> {
    let limit = q.limit.unwrap_or(10_000).clamp(1, 50_000);
    let rows = series_service::all_series_articles(&state.pool, limit).await?;
    Ok(Json(rows))
}

// --- Series context for article navigation (DAG-based) ---

#[utoipa::path(get, path = "/api/v1/series/context", params(("uri" = String, Query)), responses((status = 200)))]
pub async fn get_series_context(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<series_service::SeriesContextItem>>> {
    let context = series_service::get_series_context(&state.pool, &uri).await?;
    Ok(Json(context))
}

// --- Series tree (full hierarchy) ---

#[utoipa::path(get, path = "/api/v1/series/{id}/tree", params(("id" = String, Path)), responses((status = 200)))]
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

#[utoipa::path(put, path = "/api/v1/series/{id}/articles/reorder", params(("id" = String, Path)), responses((status = 200)), security(("bearer" = [])))]
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

#[utoipa::path(put, path = "/api/v1/series/{id}/children/reorder", params(("id" = String, Path)), responses((status = 200)), security(("bearer" = [])))]
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
