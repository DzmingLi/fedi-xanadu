use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::Result;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Listing {
    pub id: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub author_reputation: i32,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub institution: String,
    pub department: Option<String>,
    pub location: Option<String>,
    pub contact_email: Option<String>,
    pub contact_url: Option<String>,
    pub compensation: Option<String>,
    pub deadline: Option<NaiveDate>,
    pub is_open: bool,
    pub required_tags: sqlx::types::Json<Vec<String>>,
    pub preferred_tags: sqlx::types::Json<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateListing {
    pub title: String,
    pub description: Option<String>,
    pub kind: String,
    pub institution: String,
    pub department: Option<String>,
    pub location: Option<String>,
    pub contact_email: Option<String>,
    pub contact_url: Option<String>,
    pub compensation: Option<String>,
    pub deadline: Option<NaiveDate>,
    pub required_tags: Vec<String>,
    pub preferred_tags: Vec<String>,
}

const VALID_KINDS: &[&str] = &["phd", "masters", "ra", "postdoc", "intern", "faculty", "other"];

const LISTING_SELECT: &str = "\
    SELECT l.id, l.did, p.handle AS author_handle, COALESCE(p.reputation, 0) AS author_reputation, \
    l.title, l.description, l.kind, l.institution, l.department, l.location, \
    l.contact_email, l.contact_url, l.compensation, l.deadline, l.is_open, \
    COALESCE((SELECT jsonb_agg(tag_id) FROM listing_required_tags WHERE listing_id = l.id), '[]'::jsonb) AS required_tags, \
    COALESCE((SELECT jsonb_agg(tag_id) FROM listing_preferred_tags WHERE listing_id = l.id), '[]'::jsonb) AS preferred_tags, \
    l.created_at, l.updated_at \
    FROM listings l \
    LEFT JOIN profiles p ON l.did = p.did";

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

pub async fn create_listing(pool: &PgPool, id: &str, did: &str, input: &CreateListing) -> Result<Listing> {
    if !VALID_KINDS.contains(&input.kind.as_str()) {
        return Err(crate::Error::BadRequest(format!(
            "invalid kind: {}. Must be one of: {}", input.kind, VALID_KINDS.join(", ")
        )));
    }

    sqlx::query(
        "INSERT INTO listings (id, did, title, description, kind, institution, department, \
         location, contact_email, contact_url, compensation, deadline) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(id)
    .bind(did)
    .bind(&input.title)
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(&input.kind)
    .bind(&input.institution)
    .bind(&input.department)
    .bind(&input.location)
    .bind(&input.contact_email)
    .bind(&input.contact_url)
    .bind(&input.compensation)
    .bind(&input.deadline)
    .execute(pool)
    .await?;

    // Insert tags
    for tag in &input.required_tags {
        let _ = sqlx::query("INSERT INTO listing_required_tags (listing_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id).bind(tag).execute(pool).await;
    }
    for tag in &input.preferred_tags {
        let _ = sqlx::query("INSERT INTO listing_preferred_tags (listing_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id).bind(tag).execute(pool).await;
    }

    get_listing(pool, id).await
}

pub async fn get_listing(pool: &PgPool, id: &str) -> Result<Listing> {
    sqlx::query_as::<_, Listing>(&format!("{LISTING_SELECT} WHERE l.id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(crate::Error::NotFound { entity: "listing", id: id.to_string() })
}

pub async fn list_listings(
    pool: &PgPool,
    kind: Option<&str>,
    tag: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Listing>> {
    let mut conditions = vec!["l.is_open = true".to_string()];
    let mut bind_idx = 1;

    if kind.is_some() {
        bind_idx += 1;
        conditions.push(format!("l.kind = ${bind_idx}"));
    }

    if tag.is_some() {
        bind_idx += 1;
        conditions.push(format!(
            "(EXISTS (SELECT 1 FROM listing_required_tags rt WHERE rt.listing_id = l.id AND rt.tag_id = ${bind_idx}) \
             OR EXISTS (SELECT 1 FROM listing_preferred_tags pt WHERE pt.listing_id = l.id AND pt.tag_id = ${bind_idx}))"
        ));
    }

    let where_clause = conditions.join(" AND ");
    let sql = format!(
        "{LISTING_SELECT} WHERE {where_clause} ORDER BY l.created_at DESC LIMIT $1 OFFSET ${next_limit}",
        next_limit = bind_idx + 1,
    );

    let mut query = sqlx::query_as::<_, Listing>(&sql).bind(limit);
    if let Some(k) = kind {
        query = query.bind(k);
    }
    if let Some(t) = tag {
        query = query.bind(t);
    }
    query = query.bind(offset);

    let rows = query.fetch_all(pool).await?;
    Ok(rows)
}

pub async fn list_my_listings(pool: &PgPool, did: &str) -> Result<Vec<Listing>> {
    let sql = format!("{LISTING_SELECT} WHERE l.did = $1 ORDER BY l.created_at DESC");
    let rows = sqlx::query_as::<_, Listing>(&sql)
        .bind(did)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn update_listing(pool: &PgPool, id: &str, did: &str, input: &CreateListing) -> Result<Listing> {
    // Verify ownership
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM listings WHERE id = $1")
        .bind(id).fetch_optional(pool).await?;
    match owner {
        Some(ref d) if d != did => return Err(crate::Error::Forbidden { action: "edit listing" }),
        None => return Err(crate::Error::NotFound { entity: "listing", id: id.to_string() }),
        _ => {}
    }

    sqlx::query(
        "UPDATE listings SET title = $1, description = $2, kind = $3, institution = $4, \
         department = $5, location = $6, contact_email = $7, contact_url = $8, \
         compensation = $9, deadline = $10, updated_at = NOW() WHERE id = $11",
    )
    .bind(&input.title)
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(&input.kind)
    .bind(&input.institution)
    .bind(&input.department)
    .bind(&input.location)
    .bind(&input.contact_email)
    .bind(&input.contact_url)
    .bind(&input.compensation)
    .bind(&input.deadline)
    .bind(id)
    .execute(pool)
    .await?;

    // Replace tags
    sqlx::query("DELETE FROM listing_required_tags WHERE listing_id = $1").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM listing_preferred_tags WHERE listing_id = $1").bind(id).execute(pool).await?;
    for tag in &input.required_tags {
        let _ = sqlx::query("INSERT INTO listing_required_tags (listing_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id).bind(tag).execute(pool).await;
    }
    for tag in &input.preferred_tags {
        let _ = sqlx::query("INSERT INTO listing_preferred_tags (listing_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id).bind(tag).execute(pool).await;
    }

    get_listing(pool, id).await
}

pub async fn close_listing(pool: &PgPool, id: &str, did: &str) -> Result<()> {
    let result = sqlx::query("UPDATE listings SET is_open = false, updated_at = NOW() WHERE id = $1 AND did = $2")
        .bind(id).bind(did).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "listing", id: id.to_string() });
    }
    Ok(())
}

pub async fn reopen_listing(pool: &PgPool, id: &str, did: &str) -> Result<()> {
    sqlx::query("UPDATE listings SET is_open = true, updated_at = NOW() WHERE id = $1 AND did = $2")
        .bind(id).bind(did).execute(pool).await?;
    Ok(())
}

pub async fn delete_listing(pool: &PgPool, id: &str, did: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM listings WHERE id = $1 AND did = $2")
        .bind(id).bind(did).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "listing", id: id.to_string() });
    }
    Ok(())
}

/// Find listings that match a user's mastered skills.
/// Returns listings sorted by match quality (required match % desc, then preferred match).
pub async fn match_for_user(pool: &PgPool, did: &str, limit: i64) -> Result<Vec<Listing>> {
    let sql = format!(
        r#"
WITH user_mastered AS (
    SELECT DISTINCT t2.id AS tag_id FROM user_skills us JOIN tag_labels t1 ON t1.id = us.tag_id JOIN tag_labels t2 ON t2.group_id = t1.group_id WHERE us.did = $1 AND us.status = 'mastered'
),
scored AS (
    SELECT l.id,
        -- Required match ratio
        CASE WHEN req.total = 0 THEN 1.0
             ELSE COALESCE(req_met.cnt, 0)::float / req.total::float
        END AS req_ratio,
        -- Preferred match count
        COALESCE(pref_met.cnt, 0) AS pref_count
    FROM listings l
    LEFT JOIN (SELECT listing_id, COUNT(*) AS total FROM listing_required_tags GROUP BY listing_id) req
        ON req.listing_id = l.id
    LEFT JOIN (
        SELECT rt.listing_id, COUNT(*) AS cnt FROM listing_required_tags rt
        JOIN user_mastered um ON um.tag_id = rt.tag_id GROUP BY rt.listing_id
    ) req_met ON req_met.listing_id = l.id
    LEFT JOIN (
        SELECT pt.listing_id, COUNT(*) AS cnt FROM listing_preferred_tags pt
        JOIN user_mastered um ON um.tag_id = pt.tag_id GROUP BY pt.listing_id
    ) pref_met ON pref_met.listing_id = l.id
    WHERE l.is_open = true
      AND (l.deadline IS NULL OR l.deadline >= CURRENT_DATE)
)
{LISTING_SELECT}
JOIN scored s ON s.id = l.id
WHERE s.req_ratio > 0
ORDER BY s.req_ratio DESC, s.pref_count DESC, l.created_at DESC
LIMIT $2
"#
    );

    let rows = sqlx::query_as::<_, Listing>(&sql)
        .bind(did)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}
