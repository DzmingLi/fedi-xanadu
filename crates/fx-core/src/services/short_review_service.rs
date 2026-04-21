//! Short reviews on books and book series.
//!
//! A short review is just a brief text body — "one sentence about this book".
//! Star ratings live independently in `book_ratings` / `book_series_ratings`
//! (mirrored from `at.nightbo.book.rating` / `at.nightbo.bookseries.rating`);
//! the UI overlays the user's rating onto their short review at display time.
//! A user can have one without the other — a rating-only, a text-only, or
//! both — and the two are edited and deleted independently.
//!
//! PDS records: `at.nightbo.book.shortReview` (rkey = bookId),
//! `at.nightbo.bookseries.shortReview` (rkey = seriesId). Tables below are
//! the local mirror for aggregation, visibility filtering, and pagination.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ---- DB rows -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookShortReview {
    pub id: String, // at-uri
    pub did: String,
    pub book_id: String,
    pub edition_id: Option<String>,
    pub body: String,
    pub lang: Option<String>,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(default)]
    pub author_handle: Option<String>,
    #[sqlx(default)]
    pub author_display_name: Option<String>,
    #[sqlx(default)]
    pub author_avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookSeriesShortReview {
    pub id: String, // at-uri
    pub did: String,
    pub series_id: String,
    pub body: String,
    pub lang: Option<String>,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(default)]
    pub author_handle: Option<String>,
    #[sqlx(default)]
    pub author_display_name: Option<String>,
    #[sqlx(default)]
    pub author_avatar: Option<String>,
}

// ---- Request -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateBookShortReview {
    pub body: String,
    #[serde(default)]
    pub edition_id: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateSeriesShortReview {
    pub body: String,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
}

// ---- Book short reviews --------------------------------------------------

pub async fn upsert_book_short_review(
    pool: &PgPool,
    at_uri: &str,
    did: &str,
    book_id: &str,
    input: &CreateBookShortReview,
) -> crate::Result<BookShortReview> {
    validate_body(&input.body)?;
    let visibility = normalize_visibility(input.visibility.as_deref())?;

    sqlx::query(
        "INSERT INTO book_short_reviews \
           (id, did, book_id, edition_id, body, lang, visibility) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (did, book_id) DO UPDATE SET \
           id         = EXCLUDED.id, \
           edition_id = EXCLUDED.edition_id, \
           body       = EXCLUDED.body, \
           lang       = EXCLUDED.lang, \
           visibility = EXCLUDED.visibility, \
           updated_at = NOW()",
    )
    .bind(at_uri)
    .bind(did)
    .bind(book_id)
    .bind(&input.edition_id)
    .bind(&input.body)
    .bind(&input.lang)
    .bind(visibility)
    .execute(pool).await?;

    get_my_book_short_review(pool, did, book_id).await?
        .ok_or_else(|| crate::Error::Internal("upsert_book_short_review: missing row after upsert".into()))
}

pub async fn get_my_book_short_review(
    pool: &PgPool,
    did: &str,
    book_id: &str,
) -> crate::Result<Option<BookShortReview>> {
    let row = sqlx::query_as::<_, BookShortReview>(
        "SELECT sr.id, sr.did, sr.book_id, sr.edition_id, sr.body, \
                sr.lang, sr.visibility, sr.created_at, sr.updated_at, \
                pu.handle AS author_handle, pu.display_name AS author_display_name, pu.avatar_url AS author_avatar \
           FROM book_short_reviews sr \
           LEFT JOIN platform_users pu ON pu.did = sr.did \
          WHERE sr.did = $1 AND sr.book_id = $2",
    )
    .bind(did).bind(book_id)
    .fetch_optional(pool).await?;
    Ok(row)
}

pub async fn list_book_short_reviews(
    pool: &PgPool,
    book_id: &str,
    viewer_did: Option<&str>,
    limit: i64,
    offset: i64,
) -> crate::Result<Vec<BookShortReview>> {
    // visibility filter:
    //   public    → always visible
    //   followers → visible when viewer follows the author (or is the author)
    //   private   → visible only to the author
    let rows = sqlx::query_as::<_, BookShortReview>(
        "SELECT sr.id, sr.did, sr.book_id, sr.edition_id, sr.body, \
                sr.lang, sr.visibility, sr.created_at, sr.updated_at, \
                pu.handle AS author_handle, pu.display_name AS author_display_name, pu.avatar_url AS author_avatar \
           FROM book_short_reviews sr \
           LEFT JOIN platform_users pu ON pu.did = sr.did \
          WHERE sr.book_id = $1 \
            AND ( \
                  sr.visibility = 'public' \
               OR ($2::text IS NOT NULL AND sr.did = $2) \
               OR (sr.visibility = 'followers' AND $2::text IS NOT NULL AND EXISTS \
                     (SELECT 1 FROM user_follows uf WHERE uf.did = $2 AND uf.follows_did = sr.did)) \
                ) \
          ORDER BY sr.updated_at DESC \
          LIMIT $3 OFFSET $4",
    )
    .bind(book_id)
    .bind(viewer_did)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool).await?;
    Ok(rows)
}

pub async fn count_book_short_reviews(pool: &PgPool, book_id: &str) -> crate::Result<i64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM book_short_reviews WHERE book_id = $1 AND visibility = 'public'",
    )
    .bind(book_id)
    .fetch_one(pool).await?;
    Ok(n)
}

pub async fn delete_book_short_review(pool: &PgPool, did: &str, book_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM book_short_reviews WHERE did = $1 AND book_id = $2")
        .bind(did).bind(book_id)
        .execute(pool).await?;
    Ok(())
}

// ---- Series short reviews ------------------------------------------------

pub async fn upsert_series_short_review(
    pool: &PgPool,
    at_uri: &str,
    did: &str,
    series_id: &str,
    input: &CreateSeriesShortReview,
) -> crate::Result<BookSeriesShortReview> {
    validate_body(&input.body)?;
    let visibility = normalize_visibility(input.visibility.as_deref())?;

    sqlx::query(
        "INSERT INTO book_series_short_reviews \
           (id, did, series_id, body, lang, visibility) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (did, series_id) DO UPDATE SET \
           id         = EXCLUDED.id, \
           body       = EXCLUDED.body, \
           lang       = EXCLUDED.lang, \
           visibility = EXCLUDED.visibility, \
           updated_at = NOW()",
    )
    .bind(at_uri)
    .bind(did)
    .bind(series_id)
    .bind(&input.body)
    .bind(&input.lang)
    .bind(visibility)
    .execute(pool).await?;

    get_my_series_short_review(pool, did, series_id).await?
        .ok_or_else(|| crate::Error::Internal("upsert_series_short_review: missing row after upsert".into()))
}

pub async fn get_my_series_short_review(
    pool: &PgPool,
    did: &str,
    series_id: &str,
) -> crate::Result<Option<BookSeriesShortReview>> {
    let row = sqlx::query_as::<_, BookSeriesShortReview>(
        "SELECT sr.id, sr.did, sr.series_id, sr.body, sr.lang, \
                sr.visibility, sr.created_at, sr.updated_at, \
                pu.handle AS author_handle, pu.display_name AS author_display_name, pu.avatar_url AS author_avatar \
           FROM book_series_short_reviews sr \
           LEFT JOIN platform_users pu ON pu.did = sr.did \
          WHERE sr.did = $1 AND sr.series_id = $2",
    )
    .bind(did).bind(series_id)
    .fetch_optional(pool).await?;
    Ok(row)
}

pub async fn list_series_short_reviews(
    pool: &PgPool,
    series_id: &str,
    viewer_did: Option<&str>,
    limit: i64,
    offset: i64,
) -> crate::Result<Vec<BookSeriesShortReview>> {
    let rows = sqlx::query_as::<_, BookSeriesShortReview>(
        "SELECT sr.id, sr.did, sr.series_id, sr.body, sr.lang, \
                sr.visibility, sr.created_at, sr.updated_at, \
                pu.handle AS author_handle, pu.display_name AS author_display_name, pu.avatar_url AS author_avatar \
           FROM book_series_short_reviews sr \
           LEFT JOIN platform_users pu ON pu.did = sr.did \
          WHERE sr.series_id = $1 \
            AND ( \
                  sr.visibility = 'public' \
               OR ($2::text IS NOT NULL AND sr.did = $2) \
               OR (sr.visibility = 'followers' AND $2::text IS NOT NULL AND EXISTS \
                     (SELECT 1 FROM user_follows uf WHERE uf.did = $2 AND uf.follows_did = sr.did)) \
                ) \
          ORDER BY sr.updated_at DESC \
          LIMIT $3 OFFSET $4",
    )
    .bind(series_id)
    .bind(viewer_did)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool).await?;
    Ok(rows)
}

pub async fn count_series_short_reviews(pool: &PgPool, series_id: &str) -> crate::Result<i64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM book_series_short_reviews WHERE series_id = $1 AND visibility = 'public'",
    )
    .bind(series_id)
    .fetch_one(pool).await?;
    Ok(n)
}

pub async fn delete_series_short_review(pool: &PgPool, did: &str, series_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM book_series_short_reviews WHERE did = $1 AND series_id = $2")
        .bind(did).bind(series_id)
        .execute(pool).await?;
    Ok(())
}

// ---- helpers -------------------------------------------------------------

fn validate_body(b: &str) -> crate::Result<()> {
    if b.trim().is_empty() {
        return Err(crate::Error::BadRequest("short review body cannot be empty".into()));
    }
    // The lexicon caps at 500 chars; enforce on the server side too.
    if b.chars().count() > 500 {
        return Err(crate::Error::BadRequest("short review body exceeds 500 characters".into()));
    }
    Ok(())
}

fn normalize_visibility(v: Option<&str>) -> crate::Result<&'static str> {
    match v.unwrap_or("public") {
        "public"    => Ok("public"),
        "followers" => Ok("followers"),
        "private"   => Ok("private"),
        other => Err(crate::Error::BadRequest(format!("unknown visibility: {other}"))),
    }
}
