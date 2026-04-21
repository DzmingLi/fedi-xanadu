use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::services::book_service::BookRatingStats;

// ---- DB row / response ---------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookSeries {
    pub id: String,
    #[ts(type = "Record<string, string>")]
    pub title: sqlx::types::Json<HashMap<String, String>>,
    #[ts(type = "Record<string, string>")]
    pub subtitle: sqlx::types::Json<HashMap<String, String>>,
    #[ts(type = "Record<string, string>")]
    pub description: sqlx::types::Json<HashMap<String, String>>,
    pub cover_url: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

/// Short reference to a series — used as a "part of …" badge on member books.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookSeriesRef {
    pub id: String,
    #[ts(type = "Record<string, string>")]
    pub title: sqlx::types::Json<HashMap<String, String>>,
    pub position: i16,
}

/// Member of a series with list-card stats. Mirrors `BookListItem` shape plus
/// the series-specific position.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BookSeriesMemberItem {
    pub id: String,
    pub title: sqlx::types::Json<HashMap<String, String>>,
    pub subtitle: sqlx::types::Json<HashMap<String, String>>,
    pub authors: Vec<String>,
    pub description: sqlx::types::Json<HashMap<String, String>>,
    pub cover_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub avg_rating: f64,
    pub rating_count: i64,
    pub position: i16,
}

/// One line of a series list page: the series plus member-derived aggregates.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BookSeriesListItem {
    pub id: String,
    pub title: sqlx::types::Json<HashMap<String, String>>,
    pub subtitle: sqlx::types::Json<HashMap<String, String>>,
    pub description: sqlx::types::Json<HashMap<String, String>>,
    pub cover_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub member_count: i64,
    pub member_avg_rating: f64,
    pub member_rating_count: i64,
    pub series_avg_rating: f64,
    pub series_rating_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookSeriesEditLog {
    pub id: String,
    pub series_id: String,
    pub editor_did: String,
    pub editor_handle: Option<String>,
    #[ts(type = "Record<string, unknown>")]
    pub old_data: sqlx::types::Json<serde_json::Value>,
    #[ts(type = "Record<string, unknown>")]
    pub new_data: sqlx::types::Json<serde_json::Value>,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

// ---- Request -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateBookSeries {
    pub id: String,
    #[ts(type = "Record<string, string>")]
    pub title: HashMap<String, String>,
    #[serde(default)]
    #[ts(type = "Record<string, string> | undefined")]
    pub subtitle: Option<HashMap<String, String>>,
    #[serde(default)]
    #[ts(type = "Record<string, string> | undefined")]
    pub description: Option<HashMap<String, String>>,
    #[serde(default)]
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct UpdateBookSeries {
    #[serde(default)]
    #[ts(type = "Record<string, string> | undefined")]
    pub title: Option<HashMap<String, String>>,
    #[serde(default)]
    #[ts(type = "Record<string, string> | undefined")]
    pub subtitle: Option<HashMap<String, String>>,
    #[serde(default)]
    #[ts(type = "Record<string, string> | undefined")]
    pub description: Option<HashMap<String, String>>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
}

// ---- CRUD ----------------------------------------------------------------

pub async fn create_series(
    pool: &PgPool,
    input: &CreateBookSeries,
    created_by: &str,
) -> crate::Result<BookSeries> {
    let title = sqlx::types::Json(&input.title);
    let empty: HashMap<String, String> = HashMap::new();
    let subtitle = sqlx::types::Json(input.subtitle.as_ref().unwrap_or(&empty));
    let description = sqlx::types::Json(input.description.as_ref().unwrap_or(&empty));

    sqlx::query(
        "INSERT INTO book_series (id, title, subtitle, description, cover_url, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&input.id)
    .bind(title)
    .bind(subtitle)
    .bind(description)
    .bind(&input.cover_url)
    .bind(created_by)
    .execute(pool)
    .await?;

    get_series(pool, &input.id).await
}

pub async fn get_series(pool: &PgPool, id: &str) -> crate::Result<BookSeries> {
    sqlx::query_as::<_, BookSeries>(
        "SELECT id, title, subtitle, description, cover_url, created_by, created_at \
           FROM book_series \
          WHERE id = $1 AND removed_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound { entity: "book_series", id: id.to_string() })
}

pub async fn list_series(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> crate::Result<Vec<BookSeriesListItem>> {
    let rows = sqlx::query_as::<_, BookSeriesListItem>(
        "SELECT s.id, s.title, s.subtitle, s.description, s.cover_url, s.created_at, \
                COALESCE(mc.cnt, 0)                  AS member_count, \
                COALESCE(mr.avg, 0)                  AS member_avg_rating, \
                COALESCE(mr.cnt, 0)                  AS member_rating_count, \
                COALESCE(sr.avg, 0)                  AS series_avg_rating, \
                COALESCE(sr.cnt, 0)                  AS series_rating_count \
           FROM book_series s \
           LEFT JOIN (SELECT series_id, COUNT(*) AS cnt \
                        FROM book_series_members GROUP BY series_id) mc \
                  ON mc.series_id = s.id \
           LEFT JOIN (SELECT m.series_id, AVG(br.rating)::float8 AS avg, COUNT(*) AS cnt \
                        FROM book_series_members m \
                        JOIN book_ratings br ON br.book_id = m.book_id \
                       GROUP BY m.series_id) mr \
                  ON mr.series_id = s.id \
           LEFT JOIN (SELECT series_id, AVG(rating)::float8 AS avg, COUNT(*) AS cnt \
                        FROM book_series_ratings GROUP BY series_id) sr \
                  ON sr.series_id = s.id \
          WHERE s.removed_at IS NULL \
          ORDER BY s.created_at DESC \
          LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn update_series(
    pool: &PgPool,
    id: &str,
    input: &UpdateBookSeries,
    editor_did: &str,
) -> crate::Result<BookSeries> {
    let mut tx = pool.begin().await?;

    // Snapshot current row for the edit log.
    let current = sqlx::query_as::<_, BookSeries>(
        "SELECT id, title, subtitle, description, cover_url, created_by, created_at \
           FROM book_series WHERE id = $1 AND removed_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| crate::Error::NotFound { entity: "book_series", id: id.to_string() })?;

    let mut old_data = serde_json::Map::new();
    let mut new_data = serde_json::Map::new();

    if let Some(title) = &input.title {
        old_data.insert("title".into(), serde_json::to_value(&*current.title).unwrap_or_default());
        new_data.insert("title".into(), serde_json::to_value(title).unwrap_or_default());
        sqlx::query("UPDATE book_series SET title = $1 WHERE id = $2")
            .bind(sqlx::types::Json(title))
            .bind(id)
            .execute(&mut *tx).await?;
    }
    if let Some(subtitle) = &input.subtitle {
        old_data.insert("subtitle".into(), serde_json::to_value(&*current.subtitle).unwrap_or_default());
        new_data.insert("subtitle".into(), serde_json::to_value(subtitle).unwrap_or_default());
        sqlx::query("UPDATE book_series SET subtitle = $1 WHERE id = $2")
            .bind(sqlx::types::Json(subtitle))
            .bind(id)
            .execute(&mut *tx).await?;
    }
    if let Some(description) = &input.description {
        old_data.insert("description".into(), serde_json::to_value(&*current.description).unwrap_or_default());
        new_data.insert("description".into(), serde_json::to_value(description).unwrap_or_default());
        sqlx::query("UPDATE book_series SET description = $1 WHERE id = $2")
            .bind(sqlx::types::Json(description))
            .bind(id)
            .execute(&mut *tx).await?;
    }
    if let Some(cover_url) = &input.cover_url {
        old_data.insert("cover_url".into(), serde_json::Value::from(current.cover_url.clone()));
        new_data.insert("cover_url".into(), serde_json::Value::from(cover_url.clone()));
        sqlx::query("UPDATE book_series SET cover_url = $1 WHERE id = $2")
            .bind(cover_url)
            .bind(id)
            .execute(&mut *tx).await?;
    }

    if !new_data.is_empty() {
        let edit_id = format!("bse-{}", hex_short_id(12));
        sqlx::query(
            "INSERT INTO book_series_edit_log (id, series_id, editor_did, old_data, new_data, summary) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&edit_id)
        .bind(id)
        .bind(editor_did)
        .bind(sqlx::types::Json(serde_json::Value::Object(old_data)))
        .bind(sqlx::types::Json(serde_json::Value::Object(new_data)))
        .bind(input.summary.clone().unwrap_or_default())
        .execute(&mut *tx).await?;
    }

    tx.commit().await?;
    get_series(pool, id).await
}

pub async fn delete_series(pool: &PgPool, id: &str) -> crate::Result<()> {
    let res = sqlx::query("UPDATE book_series SET removed_at = NOW() WHERE id = $1 AND removed_at IS NULL")
        .bind(id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "book_series", id: id.to_string() });
    }
    Ok(())
}

pub async fn list_edit_history(pool: &PgPool, series_id: &str) -> crate::Result<Vec<BookSeriesEditLog>> {
    let rows = sqlx::query_as::<_, BookSeriesEditLog>(
        "SELECT l.id, l.series_id, l.editor_did, p.handle AS editor_handle, \
                l.old_data, l.new_data, l.summary, l.created_at \
           FROM book_series_edit_log l \
           LEFT JOIN platform_users p ON p.did = l.editor_did \
          WHERE l.series_id = $1 \
          ORDER BY l.created_at DESC",
    )
    .bind(series_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---- Membership ----------------------------------------------------------

pub async fn add_member(
    pool: &PgPool,
    series_id: &str,
    book_id: &str,
    position: i16,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO book_series_members (series_id, book_id, position) VALUES ($1, $2, $3) \
         ON CONFLICT (series_id, book_id) DO UPDATE SET position = EXCLUDED.position",
    )
    .bind(series_id)
    .bind(book_id)
    .bind(position)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_member(pool: &PgPool, series_id: &str, book_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM book_series_members WHERE series_id = $1 AND book_id = $2")
        .bind(series_id)
        .bind(book_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reorder_members(
    pool: &PgPool,
    series_id: &str,
    orders: &[(String, i16)],
) -> crate::Result<()> {
    let mut tx = pool.begin().await?;
    for (book_id, position) in orders {
        sqlx::query(
            "UPDATE book_series_members SET position = $3 \
              WHERE series_id = $1 AND book_id = $2",
        )
        .bind(series_id)
        .bind(book_id)
        .bind(position)
        .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Members of a series, ordered by position. Each row carries list-card
/// stats (cover, avg rating, rating count) so the series page can render
/// member books without a follow-up query per book.
pub async fn list_members(
    pool: &PgPool,
    series_id: &str,
) -> crate::Result<Vec<BookSeriesMemberItem>> {
    let rows = sqlx::query_as::<_, BookSeriesMemberItem>(
        "SELECT b.id, b.title, b.subtitle, b.authors, b.description, \
                (SELECT e.cover_url FROM book_editions e \
                  WHERE e.book_id = b.id AND e.cover_url IS NOT NULL \
                  ORDER BY e.created_at LIMIT 1) AS cover_url, \
                b.created_at, \
                COALESCE(r.avg, 0) AS avg_rating, \
                COALESCE(r.cnt, 0) AS rating_count, \
                m.position \
           FROM book_series_members m \
           JOIN books b ON b.id = m.book_id \
           LEFT JOIN (SELECT book_id, AVG(rating)::float8 AS avg, COUNT(*) AS cnt \
                        FROM book_ratings GROUP BY book_id) r ON r.book_id = b.id \
          WHERE m.series_id = $1 \
          ORDER BY m.position, b.created_at",
    )
    .bind(series_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// "Part of series X" badges for the given book. Ordered by English title
/// so the same book in multiple series gets a stable display order.
pub async fn list_series_badges_for_book(
    pool: &PgPool,
    book_id: &str,
) -> crate::Result<Vec<BookSeriesRef>> {
    let rows = sqlx::query_as::<_, BookSeriesRef>(
        "SELECT bs.id, bs.title, m.position \
           FROM book_series_members m \
           JOIN book_series bs ON bs.id = m.series_id \
          WHERE m.book_id = $1 AND bs.removed_at IS NULL \
          ORDER BY bs.title->>'en'",
    )
    .bind(book_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---- Ratings -------------------------------------------------------------

pub async fn rate_series(
    pool: &PgPool,
    series_id: &str,
    user_did: &str,
    rating: i16,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO book_series_ratings (series_id, user_did, rating) VALUES ($1, $2, $3) \
         ON CONFLICT (series_id, user_did) DO UPDATE SET rating = EXCLUDED.rating, updated_at = NOW()",
    )
    .bind(series_id)
    .bind(user_did)
    .bind(rating)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn unrate_series(pool: &PgPool, series_id: &str, user_did: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM book_series_ratings WHERE series_id = $1 AND user_did = $2")
        .bind(series_id)
        .bind(user_did)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_series_rating_stats(pool: &PgPool, series_id: &str) -> crate::Result<BookRatingStats> {
    let row = sqlx::query_as::<_, (Option<f64>, i64)>(
        "SELECT AVG(rating::float), COUNT(*) FROM book_series_ratings WHERE series_id = $1",
    )
    .bind(series_id)
    .fetch_one(pool)
    .await?;
    Ok(BookRatingStats { avg_rating: row.0.unwrap_or(0.0), rating_count: row.1 })
}

pub async fn get_member_rating_aggregate(
    pool: &PgPool,
    series_id: &str,
) -> crate::Result<BookRatingStats> {
    let row = sqlx::query_as::<_, (Option<f64>, i64)>(
        "SELECT AVG(br.rating::float), COUNT(*) \
           FROM book_series_members m \
           JOIN book_ratings br ON br.book_id = m.book_id \
          WHERE m.series_id = $1",
    )
    .bind(series_id)
    .fetch_one(pool)
    .await?;
    Ok(BookRatingStats { avg_rating: row.0.unwrap_or(0.0), rating_count: row.1 })
}

pub async fn get_user_series_rating(
    pool: &PgPool,
    series_id: &str,
    user_did: &str,
) -> crate::Result<Option<i16>> {
    let r: Option<i16> = sqlx::query_scalar(
        "SELECT rating FROM book_series_ratings WHERE series_id = $1 AND user_did = $2",
    )
    .bind(series_id)
    .bind(user_did)
    .fetch_optional(pool)
    .await?;
    Ok(r)
}

// ---- helpers -------------------------------------------------------------

fn hex_short_id(len: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let s = format!("{t:x}");
    let end = s.len().min(len);
    s[s.len() - end..].to_string()
}
