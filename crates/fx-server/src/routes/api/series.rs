use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::series_service;
use fx_core::validation;

use crate::error::{AppError, ApiResult, require_owner};
use crate::state::AppState;
use super::{Auth, UriQuery, tid};

#[derive(serde::Deserialize)]
pub(crate) struct CreateSeriesInput {
    title: String,
    description: Option<String>,
    tag_id: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct SeriesIdQuery {
    id: String,
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
    Auth(user): Auth,
    Json(input): Json<CreateSeriesInput>,
) -> ApiResult<(StatusCode, Json<series_service::SeriesRow>)> {
    let mut errors = Vec::new();
    if let Err(e) = validation::validate_title(&input.title) {
        errors.push(e);
    }
    if let Err(e) = validation::validate_tag_id(&input.tag_id) {
        errors.push(e);
    }
    if !errors.is_empty() {
        return Err(AppError(fx_core::Error::Validation(errors)));
    }

    let id = format!("s-{}", tid());

    let row = series_service::create_series(
        &state.pool,
        &id,
        &input.title,
        input.description.as_deref(),
        &input.tag_id,
        &user.did,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn get_series_detail(
    State(state): State<AppState>,
    Query(SeriesIdQuery { id }): Query<SeriesIdQuery>,
) -> ApiResult<Json<series_service::SeriesDetailResponse>> {
    let detail = series_service::get_series_detail(&state.pool, &id).await?;
    Ok(Json(detail))
}

#[derive(serde::Deserialize)]
pub(crate) struct AddSeriesArticleInput {
    series_id: String,
    article_uri: String,
}

pub async fn add_series_article(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<AddSeriesArticleInput>,
) -> ApiResult<StatusCode> {
    if let Err(e) = validation::validate_at_uri(&input.article_uri) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    let owner = series_service::get_series_owner(&state.pool, &input.series_id).await?;
    require_owner(Some(&owner), &user.did)?;

    series_service::add_series_article(&state.pool, &input.series_id, &input.article_uri).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct RemoveSeriesArticleInput {
    series_id: String,
    article_uri: String,
}

pub async fn remove_series_article(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<RemoveSeriesArticleInput>,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &input.series_id).await?;
    require_owner(Some(&owner), &user.did)?;

    series_service::remove_series_article(&state.pool, &input.series_id, &input.article_uri)
        .await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub(crate) struct AddSeriesPrereqInput {
    series_id: String,
    article_uri: String,
    prereq_article_uri: String,
}

pub async fn add_series_prereq(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<AddSeriesPrereqInput>,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &input.series_id).await?;
    require_owner(Some(&owner), &user.did)?;

    series_service::add_series_prereq(
        &state.pool,
        &input.series_id,
        &input.article_uri,
        &input.prereq_article_uri,
    )
    .await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub(crate) struct RemoveSeriesPrereqInput {
    series_id: String,
    article_uri: String,
    prereq_article_uri: String,
}

pub async fn remove_series_prereq(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<RemoveSeriesPrereqInput>,
) -> ApiResult<StatusCode> {
    let owner = series_service::get_series_owner(&state.pool, &input.series_id).await?;
    require_owner(Some(&owner), &user.did)?;

    series_service::remove_series_prereq(
        &state.pool,
        &input.series_id,
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
