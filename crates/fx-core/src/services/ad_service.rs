use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::Result;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Ad {
    pub id: String,
    pub title: String,
    pub body: Option<String>,
    pub image_url: Option<String>,
    pub link_url: String,
    pub placement: String,
    pub weight: i32,
    pub is_active: bool,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub daily_impression_cap: Option<i32>,
    pub total_impression_cap: Option<i32>,
    pub impressions: i64,
    pub clicks: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Lightweight struct returned to the frontend for display.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AdSlot {
    pub id: String,
    pub title: String,
    pub body: Option<String>,
    pub image_url: Option<String>,
    pub link_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAd {
    pub title: String,
    pub body: Option<String>,
    pub image_url: Option<String>,
    pub link_url: String,
    pub placement: Option<String>,
    pub weight: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub daily_impression_cap: Option<i32>,
    pub total_impression_cap: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAd {
    pub title: Option<String>,
    pub body: Option<String>,
    pub image_url: Option<String>,
    pub link_url: Option<String>,
    pub placement: Option<String>,
    pub weight: Option<i32>,
    pub is_active: Option<bool>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub daily_impression_cap: Option<i32>,
    pub total_impression_cap: Option<i32>,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Pick one ad for the given placement, respecting weight, caps, and schedule.
/// Uses weighted random selection in SQL.
pub async fn serve(pool: &PgPool, placement: &str) -> Result<Option<AdSlot>> {
    let ad = sqlx::query_as::<_, AdSlot>(
        r#"
        SELECT a.id, a.title, a.body, a.image_url, a.link_url
        FROM ads a
        LEFT JOIN ad_daily_impressions d
            ON d.ad_id = a.id AND d.day = CURRENT_DATE
        WHERE a.is_active = true
          AND a.placement = $1
          AND (a.starts_at IS NULL OR a.starts_at <= NOW())
          AND (a.ends_at   IS NULL OR a.ends_at   >= NOW())
          AND (a.total_impression_cap IS NULL OR a.impressions < a.total_impression_cap)
          AND (a.daily_impression_cap IS NULL OR COALESCE(d.count, 0) < a.daily_impression_cap)
          AND a.weight > 0
        ORDER BY random() * (1.0 / a.weight)
        LIMIT 1
        "#,
    )
    .bind(placement)
    .fetch_optional(pool)
    .await?;

    // Record impression asynchronously (best-effort)
    if let Some(ref slot) = ad {
        let _ = record_impression(pool, &slot.id).await;
    }

    Ok(ad)
}

async fn record_impression(pool: &PgPool, ad_id: &str) -> Result<()> {
    sqlx::query("UPDATE ads SET impressions = impressions + 1 WHERE id = $1")
        .bind(ad_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO ad_daily_impressions (ad_id, day, count) VALUES ($1, CURRENT_DATE, 1) \
         ON CONFLICT (ad_id, day) DO UPDATE SET count = ad_daily_impressions.count + 1",
    )
    .bind(ad_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Record a click.
pub async fn record_click(pool: &PgPool, ad_id: &str) -> Result<()> {
    let result = sqlx::query("UPDATE ads SET clicks = clicks + 1 WHERE id = $1")
        .bind(ad_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "ad", id: ad_id.to_string() });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Admin CRUD
// ---------------------------------------------------------------------------

pub async fn create(pool: &PgPool, id: &str, input: &CreateAd) -> Result<Ad> {
    sqlx::query(
        "INSERT INTO ads (id, title, body, image_url, link_url, placement, weight, \
         starts_at, ends_at, daily_impression_cap, total_impression_cap) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(id)
    .bind(&input.title)
    .bind(&input.body)
    .bind(&input.image_url)
    .bind(&input.link_url)
    .bind(input.placement.as_deref().unwrap_or("sidebar"))
    .bind(input.weight.unwrap_or(1))
    .bind(&input.starts_at)
    .bind(&input.ends_at)
    .bind(&input.daily_impression_cap)
    .bind(&input.total_impression_cap)
    .execute(pool)
    .await?;
    get(pool, id).await
}

pub async fn get(pool: &PgPool, id: &str) -> Result<Ad> {
    sqlx::query_as::<_, Ad>("SELECT * FROM ads WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(crate::Error::NotFound { entity: "ad", id: id.to_string() })
}

pub async fn list_all(pool: &PgPool) -> Result<Vec<Ad>> {
    let rows = sqlx::query_as::<_, Ad>("SELECT * FROM ads ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn update(pool: &PgPool, id: &str, input: &UpdateAd) -> Result<Ad> {
    // Fetch existing to merge
    let existing = get(pool, id).await?;

    sqlx::query(
        "UPDATE ads SET title = $1, body = $2, image_url = $3, link_url = $4, \
         placement = $5, weight = $6, is_active = $7, \
         starts_at = $8, ends_at = $9, \
         daily_impression_cap = $10, total_impression_cap = $11, \
         updated_at = NOW() \
         WHERE id = $12",
    )
    .bind(input.title.as_deref().unwrap_or(&existing.title))
    .bind(input.body.as_ref().or(existing.body.as_ref()))
    .bind(input.image_url.as_ref().or(existing.image_url.as_ref()))
    .bind(input.link_url.as_deref().unwrap_or(&existing.link_url))
    .bind(input.placement.as_deref().unwrap_or(&existing.placement))
    .bind(input.weight.unwrap_or(existing.weight))
    .bind(input.is_active.unwrap_or(existing.is_active))
    .bind(input.starts_at.or(existing.starts_at))
    .bind(input.ends_at.or(existing.ends_at))
    .bind(input.daily_impression_cap.or(existing.daily_impression_cap))
    .bind(input.total_impression_cap.or(existing.total_impression_cap))
    .bind(id)
    .execute(pool)
    .await?;

    get(pool, id).await
}

pub async fn delete(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM ads WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "ad", id: id.to_string() });
    }
    Ok(())
}
