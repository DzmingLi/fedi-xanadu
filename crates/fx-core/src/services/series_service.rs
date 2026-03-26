use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::error::Error;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub tag_id: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesListRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub tag_id: String,
    pub tag_name: String,
    pub tag_names: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesArticleRow {
    pub series_id: String,
    pub article_uri: String,
    pub title: String,
    pub description: String,
    pub lang: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesPrereqRow {
    pub article_uri: String,
    pub prereq_article_uri: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesDetailResponse {
    pub series: SeriesRow,
    pub articles: Vec<SeriesArticleRow>,
    pub prereqs: Vec<SeriesPrereqRow>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesArticleMemberRow {
    pub series_id: String,
    pub article_uri: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesContextItem {
    pub series_id: String,
    pub series_title: String,
    pub total: i64,
    pub prev: Vec<SeriesNavItem>,
    pub next: Vec<SeriesNavItem>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesNavItem {
    pub article_uri: String,
    pub title: String,
}

pub async fn list_series(pool: &PgPool, limit: i64) -> crate::Result<Vec<SeriesListRow>> {
    let rows = sqlx::query_as::<_, SeriesListRow>(
        "SELECT s.id, s.title, s.description, s.tag_id, t.name AS tag_name, t.names AS tag_names, s.created_by, s.created_at \
         FROM series s JOIN tags t ON s.tag_id = t.id ORDER BY s.created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn create_series(
    pool: &PgPool,
    id: &str,
    title: &str,
    description: Option<&str>,
    tag_id: &str,
    created_by: &str,
) -> crate::Result<SeriesRow> {
    sqlx::query(
        "INSERT INTO series (id, title, description, tag_id, created_by) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(title)
    .bind(description)
    .bind(tag_id)
    .bind(created_by)
    .execute(pool)
    .await?;

    let row = sqlx::query_as::<_, SeriesRow>(
        "SELECT id, title, description, tag_id, created_by, created_at FROM series WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_series_detail(pool: &PgPool, id: &str) -> crate::Result<SeriesDetailResponse> {
    let series = sqlx::query_as::<_, SeriesRow>(
        "SELECT id, title, description, tag_id, created_by, created_at FROM series WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| Error::NotFound {
        entity: "series",
        id: id.to_string(),
    })?;

    let articles = sqlx::query_as::<_, SeriesArticleRow>(
        "SELECT sa.series_id, sa.article_uri, a.title, COALESCE(a.description, '') AS description, a.lang \
         FROM series_articles sa JOIN articles a ON sa.article_uri = a.at_uri \
         WHERE sa.series_id = $1",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let prereqs = sqlx::query_as::<_, SeriesPrereqRow>(
        "SELECT article_uri, prereq_article_uri FROM series_article_prereqs WHERE series_id = $1",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(SeriesDetailResponse {
        series,
        articles,
        prereqs,
    })
}

pub async fn add_series_article(
    pool: &PgPool,
    series_id: &str,
    article_uri: &str,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO series_articles (series_id, article_uri) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(series_id)
    .bind(article_uri)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_series_article(
    pool: &PgPool,
    series_id: &str,
    article_uri: &str,
) -> crate::Result<()> {
    // Also remove prereq edges involving this article
    sqlx::query(
        "DELETE FROM series_article_prereqs WHERE series_id = $1 AND (article_uri = $2 OR prereq_article_uri = $2)",
    )
    .bind(series_id)
    .bind(article_uri)
    .execute(pool)
    .await?;

    sqlx::query("DELETE FROM series_articles WHERE series_id = $1 AND article_uri = $2")
        .bind(series_id)
        .bind(article_uri)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn add_series_prereq(
    pool: &PgPool,
    series_id: &str,
    article_uri: &str,
    prereq_article_uri: &str,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO series_article_prereqs (series_id, article_uri, prereq_article_uri) \
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(series_id)
    .bind(article_uri)
    .bind(prereq_article_uri)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_series_prereq(
    pool: &PgPool,
    series_id: &str,
    article_uri: &str,
    prereq_article_uri: &str,
) -> crate::Result<()> {
    sqlx::query(
        "DELETE FROM series_article_prereqs \
         WHERE series_id = $1 AND article_uri = $2 AND prereq_article_uri = $3",
    )
    .bind(series_id)
    .bind(article_uri)
    .bind(prereq_article_uri)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_series_owner(pool: &PgPool, series_id: &str) -> crate::Result<String> {
    let owner = sqlx::query_scalar::<_, String>(
        "SELECT created_by FROM series WHERE id = $1",
    )
    .bind(series_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| Error::NotFound {
        entity: "series",
        id: series_id.to_string(),
    })?;
    Ok(owner)
}

pub async fn all_series_articles(pool: &PgPool, limit: i64) -> crate::Result<Vec<SeriesArticleMemberRow>> {
    let rows = sqlx::query_as::<_, SeriesArticleMemberRow>(
        "SELECT series_id, article_uri FROM series_articles LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_series_context(
    pool: &PgPool,
    article_uri: &str,
) -> crate::Result<Vec<SeriesContextItem>> {
    let series_ids: Vec<String> = sqlx::query_scalar(
        "SELECT series_id FROM series_articles WHERE article_uri = $1",
    )
    .bind(article_uri)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for sid in series_ids {
        let series_title = sqlx::query_scalar::<_, String>(
            "SELECT title FROM series WHERE id = $1",
        )
        .bind(&sid)
        .fetch_optional(pool)
        .await?
        .unwrap_or_default();

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM series_articles WHERE series_id = $1",
        )
        .bind(&sid)
        .fetch_one(pool)
        .await?;

        // Prev: articles that are direct prerequisites of this one
        let prev = sqlx::query_as::<_, SeriesNavItem>(
            "SELECT sp.prereq_article_uri AS article_uri, a.title \
             FROM series_article_prereqs sp \
             JOIN articles a ON a.at_uri = sp.prereq_article_uri \
             WHERE sp.series_id = $1 AND sp.article_uri = $2",
        )
        .bind(&sid)
        .bind(article_uri)
        .fetch_all(pool)
        .await?;

        // Next: articles that require this one as a prerequisite
        let next = sqlx::query_as::<_, SeriesNavItem>(
            "SELECT sp.article_uri, a.title \
             FROM series_article_prereqs sp \
             JOIN articles a ON a.at_uri = sp.article_uri \
             WHERE sp.series_id = $1 AND sp.prereq_article_uri = $2",
        )
        .bind(&sid)
        .bind(article_uri)
        .fetch_all(pool)
        .await?;

        result.push(SeriesContextItem {
            series_id: sid,
            series_title,
            total,
            prev,
            next,
        });
    }

    Ok(result)
}
