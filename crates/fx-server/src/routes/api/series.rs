use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};

use crate::error::{ApiError, ApiResult, require_owner};
use crate::state::AppState;
use super::{RequireAuth, UriQuery};

#[derive(serde::Deserialize)]
pub(crate) struct CreateSeriesInput {
    title: String,
    description: Option<String>,
    tag_id: String,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct SeriesRow {
    id: String,
    title: String,
    description: Option<String>,
    tag_id: String,
    created_by: String,
    created_at: String,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct SeriesListRow {
    id: String,
    title: String,
    description: Option<String>,
    tag_id: String,
    tag_name: String,
    created_by: String,
    created_at: String,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct SeriesArticleRow {
    series_id: String,
    article_uri: String,
    title: String,
    description: String,
    lang: String,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct SeriesPrereqRow {
    article_uri: String,
    prereq_article_uri: String,
}

pub async fn list_series(State(state): State<AppState>) -> ApiResult<Json<Vec<SeriesListRow>>> {
    let rows = sqlx::query_as::<_, SeriesListRow>(
        "SELECT s.id, s.title, s.description, s.tag_id, t.name AS tag_name, s.created_by, s.created_at \
         FROM series s JOIN tags t ON s.tag_id = t.id ORDER BY s.created_at DESC"
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn create_series(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<CreateSeriesInput>,
) -> ApiResult<(StatusCode, Json<SeriesRow>)> {
    let id = format!("s-{:016x}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() & 0xFFFFFFFFFFFFFFFF);

    sqlx::query(
        "INSERT INTO series (id, title, description, tag_id, created_by) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.tag_id)
    .bind(&did)
    .execute(&state.pool)
    .await?;

    let row = sqlx::query_as::<_, SeriesRow>("SELECT id, title, description, tag_id, created_by, created_at FROM series WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;

    Ok((StatusCode::CREATED, Json(row)))
}

#[derive(serde::Deserialize)]
pub(crate) struct SeriesIdQuery {
    id: String,
}

#[derive(serde::Serialize)]
pub(crate) struct SeriesDetailResponse {
    series: SeriesRow,
    articles: Vec<SeriesArticleRow>,
    prereqs: Vec<SeriesPrereqRow>,
}

pub async fn get_series_detail(
    State(state): State<AppState>,
    Query(SeriesIdQuery { id }): Query<SeriesIdQuery>,
) -> ApiResult<Json<SeriesDetailResponse>> {
    let series = sqlx::query_as::<_, SeriesRow>("SELECT id, title, description, tag_id, created_by, created_at FROM series WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound("series not found".into()))?;

    let articles = sqlx::query_as::<_, SeriesArticleRow>(
        "SELECT sa.series_id, sa.article_uri, a.title, COALESCE(a.description, '') AS description, a.lang \
         FROM series_articles sa JOIN articles a ON sa.article_uri = a.at_uri \
         WHERE sa.series_id = ?"
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;

    let prereqs = sqlx::query_as::<_, SeriesPrereqRow>(
        "SELECT article_uri, prereq_article_uri FROM series_article_prereqs WHERE series_id = ?"
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(SeriesDetailResponse { series, articles, prereqs }))
}

#[derive(serde::Deserialize)]
pub(crate) struct AddSeriesArticleInput {
    series_id: String,
    article_uri: String,
}

pub async fn add_series_article(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<AddSeriesArticleInput>,
) -> ApiResult<StatusCode> {
    let creator: Option<String> = sqlx::query_scalar::<_, String>("SELECT created_by FROM series WHERE id = ?")
        .bind(&input.series_id)
        .fetch_optional(&state.pool)
        .await?;

    require_owner(creator.as_deref(), &did)?;

    sqlx::query(
        "INSERT OR IGNORE INTO series_articles (series_id, article_uri) VALUES (?, ?)"
    )
    .bind(&input.series_id)
    .bind(&input.article_uri)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct RemoveSeriesArticleInput {
    series_id: String,
    article_uri: String,
}

pub async fn remove_series_article(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<RemoveSeriesArticleInput>,
) -> ApiResult<StatusCode> {
    let creator: Option<String> = sqlx::query_scalar::<_, String>("SELECT created_by FROM series WHERE id = ?")
        .bind(&input.series_id)
        .fetch_optional(&state.pool)
        .await?;

    require_owner(creator.as_deref(), &did)?;

    // Also remove prereq edges involving this article
    sqlx::query("DELETE FROM series_article_prereqs WHERE series_id = ? AND (article_uri = ? OR prereq_article_uri = ?)")
        .bind(&input.series_id)
        .bind(&input.article_uri)
        .bind(&input.article_uri)
        .execute(&state.pool)
        .await?;

    sqlx::query("DELETE FROM series_articles WHERE series_id = ? AND article_uri = ?")
        .bind(&input.series_id)
        .bind(&input.article_uri)
        .execute(&state.pool)
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
    RequireAuth(did): RequireAuth,
    Json(input): Json<AddSeriesPrereqInput>,
) -> ApiResult<StatusCode> {
    let creator: Option<String> = sqlx::query_scalar::<_, String>("SELECT created_by FROM series WHERE id = ?")
        .bind(&input.series_id)
        .fetch_optional(&state.pool)
        .await?;

    require_owner(creator.as_deref(), &did)?;

    sqlx::query(
        "INSERT OR IGNORE INTO series_article_prereqs (series_id, article_uri, prereq_article_uri) VALUES (?, ?, ?)"
    )
    .bind(&input.series_id)
    .bind(&input.article_uri)
    .bind(&input.prereq_article_uri)
    .execute(&state.pool)
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
    RequireAuth(did): RequireAuth,
    Json(input): Json<RemoveSeriesPrereqInput>,
) -> ApiResult<StatusCode> {
    let creator: Option<String> = sqlx::query_scalar::<_, String>("SELECT created_by FROM series WHERE id = ?")
        .bind(&input.series_id)
        .fetch_optional(&state.pool)
        .await?;

    require_owner(creator.as_deref(), &did)?;

    sqlx::query("DELETE FROM series_article_prereqs WHERE series_id = ? AND article_uri = ? AND prereq_article_uri = ?")
        .bind(&input.series_id)
        .bind(&input.article_uri)
        .bind(&input.prereq_article_uri)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::OK)
}

// --- All series articles (for homepage dedup) ---

#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct SeriesArticleMemberRow {
    series_id: String,
    article_uri: String,
}

pub async fn all_series_articles(State(state): State<AppState>) -> ApiResult<Json<Vec<SeriesArticleMemberRow>>> {
    let rows = sqlx::query_as::<_, SeriesArticleMemberRow>(
        "SELECT series_id, article_uri FROM series_articles"
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

// --- Series context for article navigation (DAG-based) ---

#[derive(serde::Serialize)]
pub(crate) struct SeriesContextItem {
    series_id: String,
    series_title: String,
    total: i32,
    prev: Vec<SeriesNavItem>,
    next: Vec<SeriesNavItem>,
}

#[derive(serde::Serialize, Clone)]
pub(crate) struct SeriesNavItem {
    article_uri: String,
    title: String,
}

pub async fn get_series_context(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<SeriesContextItem>>> {
    // Find which series this article belongs to
    let series_ids: Vec<String> = sqlx::query_scalar(
        "SELECT series_id FROM series_articles WHERE article_uri = ?"
    )
    .bind(&uri)
    .fetch_all(&state.pool)
    .await?;

    let mut result = Vec::new();
    for sid in series_ids {
        let series_title = sqlx::query_scalar::<_, String>(
            "SELECT title FROM series WHERE id = ?"
        )
        .bind(&sid)
        .fetch_optional(&state.pool)
        .await?
        .unwrap_or_default();

        let total = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM series_articles WHERE series_id = ?"
        )
        .bind(&sid)
        .fetch_one(&state.pool)
        .await?;

        // Prev: articles that are direct prerequisites of this one
        #[derive(sqlx::FromRow)]
        struct NavRow { article_uri: String, title: String }
        let prev_rows = sqlx::query_as::<_, NavRow>(
            "SELECT sp.prereq_article_uri AS article_uri, a.title \
             FROM series_article_prereqs sp \
             JOIN articles a ON a.at_uri = sp.prereq_article_uri \
             WHERE sp.series_id = ? AND sp.article_uri = ?"
        )
        .bind(&sid)
        .bind(&uri)
        .fetch_all(&state.pool)
        .await?;

        // Next: articles that require this one as a prerequisite
        let next_rows = sqlx::query_as::<_, NavRow>(
            "SELECT sp.article_uri, a.title \
             FROM series_article_prereqs sp \
             JOIN articles a ON a.at_uri = sp.article_uri \
             WHERE sp.series_id = ? AND sp.prereq_article_uri = ?"
        )
        .bind(&sid)
        .bind(&uri)
        .fetch_all(&state.pool)
        .await?;

        result.push(SeriesContextItem {
            series_id: sid,
            series_title,
            total,
            prev: prev_rows.into_iter().map(|r| SeriesNavItem { article_uri: r.article_uri, title: r.title }).collect(),
            next: next_rows.into_iter().map(|r| SeriesNavItem { article_uri: r.article_uri, title: r.title }).collect(),
        });
    }
    Ok(Json(result))
}
