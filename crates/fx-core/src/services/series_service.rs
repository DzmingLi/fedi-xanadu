use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::error::Error;

/// Parse a synthetic chapter URI —
/// `nightboat-chapter://{series_id}/{source_path_urlencoded}` — used to
/// address series chapters that carry no per-chapter at_uri. Returns
/// `(series_id, source_path)` when the URI matches the scheme.
pub fn parse_chapter_uri(uri: &str) -> Option<(String, String)> {
    let rest = uri.strip_prefix("nightboat-chapter://")?;
    let (series_id, encoded) = rest.split_once('/')?;
    Some((series_id.to_string(), decode_uri_component(encoded)))
}

/// Minimal percent-decode for the synthetic chapter URI path segment (the
/// encoding side uses the `urlencoding` crate in fx-server). Leaves non-UTF-8
/// bytes intact; lossy UTF-8 on the way out. We avoid a new fx-core dep for
/// this single-use decode.
fn decode_uri_component(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

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
    pub category: String,
    pub split_level: i32,
    pub is_published: bool,
    #[ts(type = "number")]
    pub vote_score: i64,
    #[ts(type = "number")]
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
    #[ts(type = "number")]
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
                s.lang, s.category, s.split_level, s.is_published, \
                s.cover_url, \
                COALESCE(v.score, 0) AS vote_score, \
                COALESCE(bk.cnt, 0) AS bookmark_count \
         FROM series s \
         LEFT JOIN profiles p ON s.created_by = p.did \
         LEFT JOIN (SELECT target_uri, SUM(value) AS score FROM votes GROUP BY target_uri) v ON v.target_uri = s.id \
         LEFT JOIN (SELECT NULL::varchar AS k, 0::bigint AS cnt WHERE FALSE) bk ON bk.k = s.id \
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
    _translation_group: Option<String>, // legacy; column dropped
    category: &str,
) -> crate::Result<SeriesRow> {
    let mut tx = pool.begin().await?;

    // pijul_node_id column remains NULLABLE in the schema while the
    // pijul-to-blob migration is in flight; we always write NULL now.
    sqlx::query(
        "INSERT INTO series (id, title, summary, summary_html, long_description, order_index, created_by, lang, category, pijul_node_id) \
         VALUES ($1, $2, $3, $4, $5, 0, $6, $7, $8, NULL)",
    )
    .bind(id)
    .bind(title)
    .bind(summary)
    .bind(summary_html)
    .bind(long_description)
    .bind(created_by)
    .bind(lang)
    .bind(category)
    .execute(&mut *tx)
    .await?;

    for input_ref in topics {
        let tag_id = super::tag_service::resolve_tag_id(&mut *tx, input_ref, created_by).await?;
        sqlx::query(
            "INSERT INTO content_topics (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(&tag_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let row = sqlx::query_as::<_, SeriesRow>(
        "SELECT s.id, s.title, s.summary, s.summary_html, s.long_description, s.order_index, s.created_by, \
                p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, \
                s.created_at, s.lang, s.category, s.split_level, s.is_published, s.cover_url \
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
                s.lang, s.category, s.split_level, s.is_published, \
                s.cover_url, \
                COALESCE(v.score, 0) AS vote_score, \
                COALESCE(bk.cnt, 0) AS bookmark_count \
         FROM series s \
         LEFT JOIN profiles p ON s.created_by = p.did \
         LEFT JOIN (SELECT target_uri, SUM(value) AS score FROM votes GROUP BY target_uri) v ON v.target_uri = s.id \
         LEFT JOIN (SELECT NULL::varchar AS k, 0::bigint AS cnt WHERE FALSE) bk ON bk.k = s.id \
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
                s.created_at, s.lang, s.category, s.split_level, s.is_published, s.cover_url \
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
        "SELECT sa.series_id, \
                article_uri(sa.repo_uri, sa.source_path) AS article_uri, \
                l.title, COALESCE(l.summary, '') AS summary, l.lang, \
                sa.order_index, sa.heading_title, sa.heading_anchor \
         FROM series_articles sa \
         JOIN articles a ON a.repo_uri = sa.repo_uri AND a.source_path = sa.source_path \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
            AND l.file_path = a.source_path \
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

/// Legacy shim: `series.translation_group` was dropped. Series-level
/// translation is not modeled in the new system (per-chapter translations via
/// `article_localizations` is the direction).
#[deprecated(note = "series.translation_group removed; model per-chapter translations")]
pub async fn resolve_series_translation_group(
    _pool: &PgPool,
    source_id: &str,
) -> crate::Result<String> {
    Ok(source_id.to_string())
}

/// Legacy shim.
#[deprecated(note = "series.translation_group removed")]
pub async fn get_series_translations(_pool: &PgPool, _id: &str) -> crate::Result<Vec<SeriesRow>> {
    Ok(vec![])
}

#[allow(dead_code)]
async fn _unused_translation_placeholder(pool: &PgPool, id: &str) -> crate::Result<Vec<SeriesRow>> {
    let rows = sqlx::query_as::<_, SeriesRow>(
        "SELECT s.id, s.title, s.summary, s.summary_html, s.long_description, s.order_index, s.created_by, \
                p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, \
                s.created_at, s.lang, s.category, s.split_level, s.is_published, s.cover_url \
         FROM series s LEFT JOIN profiles p ON p.did = s.created_by WHERE s.id = $1",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Link an *existing standalone* article to a series by its `at_uri`. Used
/// when the series already owns a separate standalone article record and we
/// want to reference it from a series's table of contents. Chapters published
/// *as part of* a series (no per-chapter at_uri) should use
/// [`add_series_chapter`] instead.
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
        "INSERT INTO series_articles (series_id, repo_uri, source_path, order_index) \
         SELECT $1, repo_uri, source_path, $3 \
         FROM article_localizations WHERE at_uri = $2 \
         ON CONFLICT DO NOTHING",
    )
    .bind(series_id)
    .bind(article_uri)
    .bind(order_index)
    .execute(pool)
    .await?;
    // repo_path column was dropped; source_path replaces it.
    let _ = repo_path;
    Ok(())
}

/// Link a chapter (no per-chapter at_uri) to a series by its composite
/// `(series_repo_uri, source_path)`. Stamps `heading_anchor` so the
/// chapter's anchor for sectionRef/TOC lookup lives directly in the DB.
/// Idempotent: re-publishing the same source_path updates order + anchor.
/// Returns the resolved order_index (reused on re-publish, else next).
pub async fn add_series_chapter(
    pool: &PgPool,
    series_id: &str,
    series_repo_uri: &str,
    source_path: &str,
    anchor: Option<&str>,
) -> crate::Result<i32> {
    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT order_index FROM series_articles \
         WHERE series_id = $1 AND repo_uri = $2 AND source_path = $3",
    )
    .bind(series_id)
    .bind(series_repo_uri)
    .bind(source_path)
    .fetch_optional(pool)
    .await?;

    let order_index: i32 = if let Some(idx) = existing {
        idx
    } else {
        let max_idx: Option<i32> = sqlx::query_scalar(
            "SELECT MAX(order_index) FROM series_articles WHERE series_id = $1",
        )
        .bind(series_id)
        .fetch_one(pool)
        .await?;
        max_idx.unwrap_or(-1) + 1
    };

    sqlx::query(
        "INSERT INTO series_articles (series_id, repo_uri, source_path, order_index, heading_anchor) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (series_id, repo_uri, source_path) DO UPDATE SET \
             heading_anchor = EXCLUDED.heading_anchor",
    )
    .bind(series_id)
    .bind(series_repo_uri)
    .bind(source_path)
    .bind(order_index)
    .bind(anchor)
    .execute(pool)
    .await?;
    Ok(order_index)
}

/// Look up synthetic article URI by its repo path within a series.
pub async fn find_article_by_repo_path(
    pool: &PgPool,
    series_id: &str,
    repo_path: &str,
) -> crate::Result<Option<String>> {
    let uri = sqlx::query_scalar::<_, String>(
        "SELECT article_uri(repo_uri, source_path) FROM series_articles \
         WHERE series_id = $1 AND source_path = $2",
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

/// Construct the canonical `repo_uri` for a series — the stable identifier
/// for the series record: `at://{creator_did}/at.nightbo.work/{series_id}`.
pub async fn series_repo_uri(pool: &PgPool, series_id: &str) -> crate::Result<String> {
    let owner = get_series_owner(pool, series_id).await?;
    Ok(format!("at://{owner}/at.nightbo.work/{series_id}"))
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
        "SELECT series_id, article_uri(repo_uri, source_path) AS article_uri \
         FROM series_articles LIMIT $1",
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
                s.created_at, s.lang, s.category, s.split_level, s.is_published, s.cover_url \
         FROM series s LEFT JOIN profiles p ON p.did = s.created_by WHERE s.id = $1",
    )
    .bind(root_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| Error::NotFound { entity: "series", id: root_id.to_string() })?;

    let articles = sqlx::query_as::<_, SeriesArticleRow>(
        "SELECT sa.series_id, \
                article_uri(sa.repo_uri, sa.source_path) AS article_uri, \
                l.title, COALESCE(l.summary, '') AS summary, l.lang, \
                sa.order_index, sa.heading_title, sa.heading_anchor \
         FROM series_articles sa \
         JOIN articles a ON a.repo_uri = sa.repo_uri AND a.source_path = sa.source_path \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
            AND l.file_path = a.source_path \
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
    // Accept either the standalone `at://` URI (linked-into-series case) or
    // the synthetic `nightboat-chapter://{series_id}/{source_path}` URI
    // (chapters published as parts of a series). Resolve to composite key.
    let (series_scope, cur_r, cur_s): (Option<String>, String, String) =
        if let Some(rest) = article_uri.strip_prefix("nightboat-chapter://") {
            let Some((sid, encoded)) = rest.split_once('/') else {
                return Ok(vec![]);
            };
            let sp = decode_uri_component(encoded);
            let series_repo_uri = series_repo_uri(pool, sid).await.unwrap_or_default();
            if series_repo_uri.is_empty() { return Ok(vec![]) };
            (Some(sid.to_string()), series_repo_uri, sp)
        } else {
            let row: Option<(String, String)> = sqlx::query_as(
                "SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1",
            )
            .bind(article_uri)
            .fetch_optional(pool)
            .await?;
            let Some((r, s)) = row else { return Ok(vec![]) };
            (None, r, s)
        };

    let series_ids: Vec<String> = if let Some(sid) = series_scope {
        vec![sid]
    } else {
        sqlx::query_scalar(
            "SELECT series_id FROM series_articles \
             WHERE repo_uri = $1 AND source_path = $2",
        )
        .bind(&cur_r).bind(&cur_s)
        .fetch_all(pool)
        .await?
    };

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
            "SELECT article_uri(sp.prereq_repo_uri, sp.prereq_source_path) AS article_uri, \
                    l.title \
             FROM series_article_prereqs sp \
             JOIN articles a \
                 ON a.repo_uri = sp.prereq_repo_uri AND a.source_path = sp.prereq_source_path \
             JOIN article_localizations l \
                 ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
                AND l.file_path = a.source_path \
             WHERE sp.series_id = $1 \
               AND sp.repo_uri = $2 AND sp.source_path = $3",
        )
        .bind(&sid).bind(&cur_r).bind(&cur_s)
        .fetch_all(pool)
        .await?;

        // Next: articles that require this one as a prerequisite
        let next = sqlx::query_as::<_, SeriesNavItem>(
            "SELECT article_uri(sp.repo_uri, sp.source_path) AS article_uri, l.title \
             FROM series_article_prereqs sp \
             JOIN articles a \
                 ON a.repo_uri = sp.repo_uri AND a.source_path = sp.source_path \
             JOIN article_localizations l \
                 ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
                AND l.file_path = a.source_path \
             WHERE sp.series_id = $1 \
               AND sp.prereq_repo_uri = $2 AND sp.prereq_source_path = $3",
        )
        .bind(&sid).bind(&cur_r).bind(&cur_s)
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
/// Chapters don't have per-chapter at_uris in the unified-series model —
/// identity is `(series_repo_uri, source_path)`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SeriesChapterInfo {
    pub source_path: String,
    pub file_path: String,
    pub content_format: String,
    pub order_index: i32,
    pub heading_anchor: Option<String>,
}

/// Get all chapters in a series, ordered by position. Returns the series's
/// `repo_uri` alongside — chapters' source files live under
/// `{blob_cache}/{uri_to_node_id(series_repo_uri)}/{source_path}`.
pub async fn get_series_chapters_for_render(
    pool: &PgPool,
    series_id: &str,
) -> crate::Result<Option<(String, Vec<SeriesChapterInfo>)>> {
    // Pull the series repo_uri from the first chapter row. (Every chapter in
    // the series shares the same repo_uri = series record at-uri.) If the
    // series exists but has no chapters we still return the repo_uri via
    // series_repo_uri() so callers can initialize the blob cache dir.
    let repo_uri: Option<String> = sqlx::query_scalar(
        "SELECT repo_uri FROM series_articles WHERE series_id = $1 LIMIT 1",
    )
    .bind(series_id)
    .fetch_optional(pool)
    .await?;

    let series_repo_uri = match repo_uri {
        Some(r) => r,
        None => {
            // Fall back to computing it from the series owner — if the series
            // itself doesn't exist this errors; if it does we return (uri, []).
            match get_series_owner(pool, series_id).await {
                Ok(_) => series_repo_uri(pool, series_id).await?,
                Err(_) => return Ok(None),
            }
        }
    };

    let chapters = sqlx::query_as::<_, SeriesChapterInfo>(
        "SELECT sa.source_path, l.file_path, l.content_format::text AS content_format, \
                sa.order_index, sa.heading_anchor \
         FROM series_articles sa \
         JOIN articles a ON a.repo_uri = sa.repo_uri AND a.source_path = sa.source_path \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
            AND l.file_path = a.source_path \
         WHERE sa.series_id = $1 \
         ORDER BY sa.order_index",
    )
    .bind(series_id)
    .fetch_all(pool)
    .await?;

    Ok(Some((series_repo_uri, chapters)))
}
