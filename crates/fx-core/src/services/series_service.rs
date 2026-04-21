use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::error::Error;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SeriesRow {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    #[sqlx(default)]
    pub summary_html: String,
    pub long_description: Option<String>,
    pub order_index: i32,
    pub created_by: String,
    #[sqlx(default)]
    pub author_handle: Option<String>,
    #[sqlx(default)]
    pub author_display_name: Option<String>,
    #[sqlx(default)]
    pub author_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
    pub lang: String,
    pub translation_group: Option<String>,
    pub category: String,
    pub split_level: i32,
    pub is_published: bool,
    #[sqlx(default)]
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SeriesListRow {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    #[sqlx(default)]
    pub summary_html: String,
    pub long_description: Option<String>,
    pub order_index: i32,
    pub created_by: String,
    pub author_handle: Option<String>,
    pub author_display_name: Option<String>,
    pub author_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
    pub lang: String,
    pub translation_group: Option<String>,
    pub category: String,
    pub split_level: i32,
    pub is_published: bool,
    pub vote_score: i64,
    pub bookmark_count: i64,
    #[sqlx(default)]
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SeriesArticleRow {
    pub series_id: String,
    pub article_uri: String,
    pub title: String,
    pub summary: String,
    pub lang: String,
    pub order_index: i32,
    pub heading_title: Option<String>,
    pub heading_anchor: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SeriesPrereqRow {
    pub article_uri: String,
    pub prereq_article_uri: String,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SeriesDetailResponse {
    pub series: SeriesRow,
    pub articles: Vec<SeriesArticleRow>,
    pub prereqs: Vec<SeriesPrereqRow>,
    pub translations: Vec<SeriesRow>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SeriesTreeNode {
    pub series: SeriesRow,
    pub articles: Vec<SeriesArticleRow>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SeriesArticleMemberRow {
    pub series_id: String,
    pub article_uri: String,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SeriesContextItem {
    pub series_id: String,
    pub series_title: String,
    pub total: i64,
    pub prev: Vec<SeriesNavItem>,
    pub next: Vec<SeriesNavItem>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SeriesNavItem {
    pub article_uri: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct SeriesHeadingRow {
    pub id: i32,
    pub series_id: String,
    pub level: i32,
    pub title: String,
    pub anchor: String,
    pub article_uri: Option<String>,
    pub parent_heading_id: Option<i32>,
    pub order_index: i32,
}

pub async fn list_series(pool: &PgPool, limit: i64) -> crate::Result<Vec<SeriesListRow>> {
    let rows = sqlx::query_as::<_, SeriesListRow>(
        "SELECT s.id, s.title, s.summary, s.summary_html, s.long_description, \
                s.order_index, s.created_by, p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, s.created_at, \
                s.lang, s.translation_group, s.category, s.split_level, s.is_published, \
                s.cover_url, \
                COALESCE(v.score, 0) AS vote_score, \
                COALESCE(bk.cnt, 0) AS bookmark_count \
         FROM series s \
         LEFT JOIN profiles p ON s.created_by = p.did \
         LEFT JOIN (SELECT target_uri, SUM(value) AS score FROM votes GROUP BY target_uri) v ON v.target_uri = s.id \
         LEFT JOIN (SELECT article_uri, COUNT(*) AS cnt FROM user_bookmarks GROUP BY article_uri) bk ON bk.article_uri = s.id \
         WHERE s.is_published = TRUE \
         ORDER BY s.created_at DESC LIMIT $1",
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
    summary: Option<&str>,
    summary_html: &str,
    long_description: Option<&str>,
    topics: &[String],
    created_by: &str,
    lang: &str,
    translation_group: Option<String>,
    category: &str,
    pijul_node_id: Option<&str>,
) -> crate::Result<SeriesRow> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO series (id, title, summary, summary_html, long_description, order_index, created_by, lang, translation_group, category, pijul_node_id) \
         VALUES ($1, $2, $3, $4, $5, 0, $6, $7, $8, $9, $10)",
    )
    .bind(id)
    .bind(title)
    .bind(summary)
    .bind(summary_html)
    .bind(long_description)
    .bind(created_by)
    .bind(lang)
    .bind(&translation_group)
    .bind(category)
    .bind(pijul_node_id)
    .execute(&mut *tx)
    .await?;

    for topic_label in topics {
        super::tag_service::ensure_tag(&mut *tx, topic_label, created_by).await?;
        sqlx::query(
            "INSERT INTO content_topics (content_uri, tag_id) \
             VALUES ($1, (SELECT tag_id FROM tag_labels WHERE id = $2)) \
             ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(topic_label)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let row = sqlx::query_as::<_, SeriesRow>(
        "SELECT s.id, s.title, s.summary, s.summary_html, s.long_description, s.order_index, s.created_by, \
                p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, \
                s.created_at, s.lang, s.translation_group, s.category, s.split_level, s.is_published, s.cover_url \
         FROM series s LEFT JOIN profiles p ON p.did = s.created_by WHERE s.id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn publish_series(pool: &PgPool, id: &str) -> crate::Result<()> {
    sqlx::query("UPDATE series SET is_published = TRUE WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn unpublish_series(pool: &PgPool, id: &str) -> crate::Result<()> {
    sqlx::query("UPDATE series SET is_published = FALSE WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// List all series by a creator (including drafts).
pub async fn list_series_by_creator(pool: &PgPool, did: &str) -> crate::Result<Vec<SeriesListRow>> {
    let rows = sqlx::query_as::<_, SeriesListRow>(
        "SELECT s.id, s.title, s.summary, s.summary_html, s.long_description, \
                s.order_index, s.created_by, p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, s.created_at, \
                s.lang, s.translation_group, s.category, s.split_level, s.is_published, \
                s.cover_url, \
                COALESCE(v.score, 0) AS vote_score, \
                COALESCE(bk.cnt, 0) AS bookmark_count \
         FROM series s \
         LEFT JOIN profiles p ON s.created_by = p.did \
         LEFT JOIN (SELECT target_uri, SUM(value) AS score FROM votes GROUP BY target_uri) v ON v.target_uri = s.id \
         LEFT JOIN (SELECT article_uri, COUNT(*) AS cnt FROM user_bookmarks GROUP BY article_uri) bk ON bk.article_uri = s.id \
         WHERE s.created_by = $1 \
         ORDER BY s.created_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_series_detail(pool: &PgPool, id: &str) -> crate::Result<SeriesDetailResponse> {
    let series = sqlx::query_as::<_, SeriesRow>(
        "SELECT s.id, s.title, s.summary, s.summary_html, s.long_description, s.order_index, s.created_by, \
                p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, \
                s.created_at, s.lang, s.translation_group, s.category, s.split_level, s.is_published, s.cover_url \
         FROM series s LEFT JOIN profiles p ON p.did = s.created_by WHERE s.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| Error::NotFound {
        entity: "series",
        id: id.to_string(),
    })?;

    let articles = sqlx::query_as::<_, SeriesArticleRow>(
        "SELECT sa.series_id, sa.article_uri, a.title, COALESCE(a.summary, '') AS summary, \
                a.lang, sa.order_index, sa.heading_title, sa.heading_anchor \
         FROM series_articles sa JOIN articles a ON sa.article_uri = a.at_uri \
         WHERE sa.series_id = $1 ORDER BY sa.order_index",
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

    let translations = get_series_translations(pool, id).await?;

    Ok(SeriesDetailResponse {
        series,
        articles,
        prereqs,
        translations,
    })
}

pub async fn resolve_series_translation_group(
    pool: &PgPool,
    source_id: &str,
) -> crate::Result<String> {
    let group: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(translation_group, id) FROM series WHERE id = $1",
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| Error::NotFound {
        entity: "series",
        id: source_id.to_string(),
    })?;

    let g = group.unwrap_or_else(|| source_id.to_string());

    sqlx::query(
        "UPDATE series SET translation_group = $1 WHERE id = $2 AND translation_group IS NULL",
    )
    .bind(&g)
    .bind(source_id)
    .execute(pool)
    .await?;

    Ok(g)
}

pub async fn get_series_translations(pool: &PgPool, id: &str) -> crate::Result<Vec<SeriesRow>> {
    let group: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(translation_group, id) FROM series WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let group = match group {
        Some(g) => g,
        None => return Ok(vec![]),
    };

    let rows = sqlx::query_as::<_, SeriesRow>(
        "SELECT s.id, s.title, s.summary, s.summary_html, s.long_description, s.order_index, s.created_by, \
                p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, \
                s.created_at, s.lang, s.translation_group, s.category, s.split_level, s.is_published, s.cover_url \
         FROM series s LEFT JOIN profiles p ON p.did = s.created_by WHERE s.translation_group = $1 AND s.id != $2 ORDER BY s.lang",
    )
    .bind(&group)
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Link an article to a series.
///
/// `repo_path` is `Some("ch1/intro.md")` when the article's source lives in
/// the series pijul repo at that path, or `None` when it's a standalone
/// article whose content is stored in its own per-article repo.
pub async fn add_series_article(
    pool: &PgPool,
    series_id: &str,
    article_uri: &str,
    repo_path: Option<&str>,
) -> crate::Result<()> {
    let order_index: i32 = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(order_index) FROM series_articles WHERE series_id = $1",
    )
    .bind(series_id)
    .fetch_one(pool)
    .await?
    .unwrap_or(-1)
    + 1;

    sqlx::query(
        "INSERT INTO series_articles (series_id, article_uri, order_index, repo_path) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(series_id)
    .bind(article_uri)
    .bind(order_index)
    .bind(repo_path)
    .execute(pool)
    .await?;
    Ok(())
}

/// Look up an article URI by its repo path within a series.
pub async fn find_article_by_repo_path(
    pool: &PgPool,
    series_id: &str,
    repo_path: &str,
) -> crate::Result<Option<String>> {
    let uri = sqlx::query_scalar::<_, String>(
        "SELECT article_uri FROM series_articles WHERE series_id = $1 AND repo_path = $2",
    )
    .bind(series_id)
    .bind(repo_path)
    .fetch_optional(pool)
    .await?;
    Ok(uri)
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

/// Build a flat tree node for a series (no children — parent series removed).
pub async fn get_series_tree(pool: &PgPool, root_id: &str) -> crate::Result<SeriesTreeNode> {
    let series = sqlx::query_as::<_, SeriesRow>(
        "SELECT s.id, s.title, s.summary, s.summary_html, s.long_description, s.order_index, s.created_by, \
                p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, \
                s.created_at, s.lang, s.translation_group, s.category, s.split_level, s.is_published, s.cover_url \
         FROM series s LEFT JOIN profiles p ON p.did = s.created_by WHERE s.id = $1",
    )
    .bind(root_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| Error::NotFound { entity: "series", id: root_id.to_string() })?;

    let articles = sqlx::query_as::<_, SeriesArticleRow>(
        "SELECT sa.series_id, sa.article_uri, a.title, COALESCE(a.summary, '') AS summary, \
                a.lang, sa.order_index, sa.heading_title, sa.heading_anchor \
         FROM series_articles sa JOIN articles a ON sa.article_uri = a.at_uri \
         WHERE sa.series_id = $1 ORDER BY sa.order_index",
    )
    .bind(root_id)
    .fetch_all(pool)
    .await?;

    Ok(SeriesTreeNode { series, articles })
}

/// Reorder articles within a series.
pub async fn reorder_series_articles(
    pool: &PgPool,
    series_id: &str,
    article_uris: &[String],
) -> crate::Result<()> {
    for (i, uri) in article_uris.iter().enumerate() {
        sqlx::query(
            "UPDATE series_articles SET order_index = $1 WHERE series_id = $2 AND article_uri = $3",
        )
        .bind(i as i32)
        .bind(series_id)
        .bind(uri)
        .execute(pool)
        .await?;
    }
    Ok(())
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

/// Info about a chapter in a series, used for virtual-document rendering.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SeriesChapterInfo {
    pub article_uri: String,
    pub order_index: i32,
    pub content_format: String,
    pub repo_path: Option<String>,
}

/// Get all chapters in the same series as the given article, ordered by position.
/// Returns None if the article doesn't belong to any series.
/// If it belongs to multiple series, returns the first one found.
pub async fn get_series_chapters_for_render(
    pool: &PgPool,
    article_uri: &str,
) -> crate::Result<Option<(String, Vec<SeriesChapterInfo>)>> {
    // Find the first series this article belongs to
    let series_id: Option<String> = sqlx::query_scalar(
        "SELECT series_id FROM series_articles WHERE article_uri = $1 LIMIT 1",
    )
    .bind(article_uri)
    .fetch_optional(pool)
    .await?;

    let Some(series_id) = series_id else {
        return Ok(None);
    };

    let chapters = sqlx::query_as::<_, SeriesChapterInfo>(
        "SELECT sa.article_uri, sa.order_index, a.content_format::text, sa.repo_path \
         FROM series_articles sa \
         JOIN articles a ON a.at_uri = sa.article_uri \
         WHERE sa.series_id = $1 \
         ORDER BY sa.order_index",
    )
    .bind(&series_id)
    .fetch_all(pool)
    .await?;

    Ok(Some((series_id, chapters)))
}
