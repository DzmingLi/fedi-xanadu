use axum::{Json, extract::State};
use serde::Serialize;

use crate::auth::Auth;
use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use fx_core::services::series_service;

#[derive(Serialize)]
pub struct CreatorStats {
    pub total_articles: i64,
    pub total_series: i64,
    pub total_drafts: i64,
    pub total_views: i64,
    pub total_comments: i64,
    pub total_bookmarks: i64,
}

#[derive(Serialize)]
pub struct ArticleStats {
    pub at_uri: String,
    pub title: String,
    pub content_format: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub views: i64,
    pub comments: i64,
    pub bookmarks: i64,
    pub votes: i64,
}

#[derive(Serialize)]
pub struct TimelinePoint {
    pub day: String,
    pub views: i64,
    pub comments: i64,
    pub bookmarks: i64,
}

/// GET /api/creator/stats
pub async fn get_stats(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<CreatorStats>> {
    let total_articles: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM articles WHERE author_did = $1 AND removed_at IS NULL AND visibility = 'public'"
    ).bind(&user.did).fetch_one(&state.pool).await?;

    let total_series: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM series WHERE created_by = $1 AND is_published = TRUE"
    ).bind(&user.did).fetch_one(&state.pool).await?;

    let total_drafts: (i64,) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM drafts WHERE did = $1) + \
         (SELECT COUNT(*) FROM series WHERE created_by = $1 AND is_published = FALSE)"
    ).bind(&user.did).fetch_one(&state.pool).await?;

    let total_views: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM article_views av \
         JOIN articles a ON a.repo_uri = av.repo_uri AND a.source_path = av.source_path \
         WHERE a.author_did = $1 AND a.removed_at IS NULL"
    ).bind(&user.did).fetch_one(&state.pool).await?;

    let total_comments: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM comments c \
         JOIN articles a ON c.content_uri = article_uri(a.repo_uri, a.source_path) \
         WHERE a.author_did = $1 AND a.removed_at IS NULL"
    ).bind(&user.did).fetch_one(&state.pool).await?;

    let total_bookmarks: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM user_bookmarks b \
         JOIN articles a ON a.repo_uri = b.repo_uri AND a.source_path = b.source_path \
         WHERE a.author_did = $1 AND a.removed_at IS NULL"
    ).bind(&user.did).fetch_one(&state.pool).await?;

    Ok(Json(CreatorStats {
        total_articles: total_articles.0,
        total_series: total_series.0,
        total_drafts: total_drafts.0,
        total_views: total_views.0,
        total_comments: total_comments.0,
        total_bookmarks: total_bookmarks.0,
    }))
}

/// GET /api/creator/articles
pub async fn list_articles(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<ArticleStats>>> {
    let rows = sqlx::query_as::<_, (String, String, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT l.at_uri, l.title, l.content_format::TEXT, a.created_at FROM articles a \
         JOIN article_localizations l \
           ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
         WHERE a.author_did = $1 AND a.removed_at IS NULL \
           AND l.at_uri IS NOT NULL AND l.file_path = a.source_path \
         ORDER BY a.created_at DESC"
    ).bind(&user.did).fetch_all(&state.pool).await?;

    let mut articles = Vec::new();
    for (uri, title, fmt, created_at) in rows {
        let views: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM article_views WHERE article_uri = $1"
        ).bind(&uri).fetch_one(&state.pool).await.unwrap_or((0,));

        let comments: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM comments WHERE content_uri = $1"
        ).bind(&uri).fetch_one(&state.pool).await.unwrap_or((0,));

        let bookmarks: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_bookmarks WHERE article_uri = $1"
        ).bind(&uri).fetch_one(&state.pool).await.unwrap_or((0,));

        let votes: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(value), 0) FROM votes WHERE target_uri = $1"
        ).bind(&uri).fetch_one(&state.pool).await.unwrap_or((0,));

        articles.push(ArticleStats {
            at_uri: uri, title, content_format: fmt, created_at,
            views: views.0, comments: comments.0, bookmarks: bookmarks.0, votes: votes.0,
        });
    }

    Ok(Json(articles))
}

/// GET /api/creator/series
pub async fn list_series(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<series_service::SeriesListRow>>> {
    let rows = series_service::list_series_by_creator(&state.pool, &user.did).await?;
    Ok(Json(rows))
}

/// GET /api/creator/timeline
pub async fn get_timeline(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<TimelinePoint>>> {
    // Last 30 days
    let rows = sqlx::query_as::<_, (chrono::NaiveDate, i64, i64, i64)>(
        "SELECT day, SUM(views)::bigint, SUM(comments)::bigint, SUM(bookmarks)::bigint \
         FROM creator_daily_stats \
         WHERE creator_did = $1 AND day >= CURRENT_DATE - INTERVAL '30 days' \
         GROUP BY day ORDER BY day"
    ).bind(&user.did).fetch_all(&state.pool).await
        .unwrap_or_default();

    let points: Vec<TimelinePoint> = rows.into_iter().map(|(day, v, c, b)| TimelinePoint {
        day: day.to_string(),
        views: v, comments: c, bookmarks: b,
    }).collect();

    Ok(Json(points))
}

/// POST /api/series/{id}/publish
pub async fn publish_series(
    State(state): State<AppState>,
    Auth(user): Auth,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<axum::http::StatusCode> {
    // Verify ownership
    let owner = series_service::get_series_owner(&state.pool, &id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "publish" }));
    }
    series_service::publish_series(&state.pool, &id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// POST /api/articles/view - record a view
pub async fn record_view(
    State(state): State<AppState>,
    Json(body): Json<RecordViewInput>,
) -> ApiResult<axum::http::StatusCode> {
    sqlx::query("INSERT INTO article_views (article_uri, viewer_did) VALUES ($1, $2)")
        .bind(&body.uri)
        .bind(&body.viewer_did)
        .execute(&state.pool)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct RecordViewInput {
    uri: String,
    viewer_did: Option<String>,
}
