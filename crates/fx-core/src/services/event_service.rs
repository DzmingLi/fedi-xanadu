use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::Result;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Event {
    pub id: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub author_reputation: i32,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub location: Option<String>,
    pub online_url: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub organizer: String,
    pub contact_email: Option<String>,
    pub contact_url: Option<String>,
    pub max_attendees: Option<i32>,
    pub is_cancelled: bool,
    pub teaches: sqlx::types::Json<Vec<String>>,
    pub prereqs: sqlx::types::Json<Vec<String>>,
    pub rsvp_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateEvent {
    pub title: String,
    pub description: Option<String>,
    pub kind: String,
    pub location: Option<String>,
    pub online_url: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub organizer: String,
    pub contact_email: Option<String>,
    pub contact_url: Option<String>,
    pub max_attendees: Option<i32>,
    pub teaches: Vec<String>,
    pub prereqs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Rsvp {
    pub event_id: String,
    pub did: String,
    pub status: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RsvpInput {
    pub status: Option<String>,
}

const VALID_KINDS: &[&str] = &["conference", "workshop", "seminar", "meetup", "hackathon"];

const EVENT_SELECT: &str = "\
    SELECT e.id, e.did, p.handle AS author_handle, COALESCE(p.reputation, 0) AS author_reputation, \
    e.title, e.description, e.kind, e.location, e.online_url, \
    e.start_time, e.end_time, e.organizer, e.contact_email, e.contact_url, \
    e.max_attendees, e.is_cancelled, \
    COALESCE((SELECT jsonb_agg(tag_id) FROM event_teaches WHERE event_id = e.id), '[]'::jsonb) AS teaches, \
    COALESCE((SELECT jsonb_agg(tag_id) FROM event_prereqs WHERE event_id = e.id), '[]'::jsonb) AS prereqs, \
    COALESCE((SELECT COUNT(*) FROM event_rsvps WHERE event_id = e.id), 0) AS rsvp_count, \
    e.created_at, e.updated_at \
    FROM events e \
    LEFT JOIN profiles p ON e.did = p.did";

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

pub async fn create_event(pool: &PgPool, id: &str, did: &str, input: &CreateEvent) -> Result<Event> {
    if !VALID_KINDS.contains(&input.kind.as_str()) {
        return Err(crate::Error::BadRequest(format!(
            "invalid kind: {}. Must be one of: {}", input.kind, VALID_KINDS.join(", ")
        )));
    }

    sqlx::query(
        "INSERT INTO events (id, did, title, description, kind, location, online_url, \
         start_time, end_time, organizer, contact_email, contact_url, max_attendees) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(id).bind(did)
    .bind(&input.title)
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(&input.kind)
    .bind(&input.location)
    .bind(&input.online_url)
    .bind(&input.start_time)
    .bind(&input.end_time)
    .bind(&input.organizer)
    .bind(&input.contact_email)
    .bind(&input.contact_url)
    .bind(&input.max_attendees)
    .execute(pool)
    .await?;

    for tag in &input.teaches {
        let _ = sqlx::query("INSERT INTO event_teaches (event_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id).bind(tag).execute(pool).await;
    }
    for tag in &input.prereqs {
        let _ = sqlx::query("INSERT INTO event_prereqs (event_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id).bind(tag).execute(pool).await;
    }

    get_event(pool, id).await
}

pub async fn get_event(pool: &PgPool, id: &str) -> Result<Event> {
    sqlx::query_as::<_, Event>(&format!("{EVENT_SELECT} WHERE e.id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(crate::Error::NotFound { entity: "event", id: id.to_string() })
}

pub async fn list_events(
    pool: &PgPool,
    kind: Option<&str>,
    tag: Option<&str>,
    upcoming: Option<bool>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Event>> {
    let mut conditions = vec!["e.is_cancelled = false".to_string()];
    let mut bind_idx = 0;

    if upcoming == Some(true) {
        conditions.push("e.start_time >= NOW()".to_string());
    } else if upcoming == Some(false) {
        conditions.push("e.start_time < NOW()".to_string());
    }

    if kind.is_some() {
        bind_idx += 1;
        conditions.push(format!("e.kind = ${bind_idx}"));
    }

    if tag.is_some() {
        bind_idx += 1;
        conditions.push(format!(
            "(EXISTS (SELECT 1 FROM event_teaches et WHERE et.event_id = e.id AND et.tag_id = ${bind_idx}))"
        ));
    }

    let order = if upcoming == Some(true) { "e.start_time ASC" } else { "e.start_time DESC" };
    let limit_idx = bind_idx + 1;
    let offset_idx = bind_idx + 2;

    let where_clause = conditions.join(" AND ");
    let sql = format!(
        "{EVENT_SELECT} WHERE {where_clause} ORDER BY {order} LIMIT ${limit_idx} OFFSET ${offset_idx}"
    );

    let mut query = sqlx::query_as::<_, Event>(&sql);
    if let Some(k) = kind {
        query = query.bind(k);
    }
    if let Some(t) = tag {
        query = query.bind(t);
    }
    query = query.bind(limit).bind(offset);

    let rows = query.fetch_all(pool).await?;
    Ok(rows)
}

pub async fn list_my_events(pool: &PgPool, did: &str) -> Result<Vec<Event>> {
    let sql = format!("{EVENT_SELECT} WHERE e.did = $1 ORDER BY e.created_at DESC");
    let rows = sqlx::query_as::<_, Event>(&sql)
        .bind(did)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn update_event(pool: &PgPool, id: &str, did: &str, input: &CreateEvent) -> Result<Event> {
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM events WHERE id = $1")
        .bind(id).fetch_optional(pool).await?;
    match owner {
        Some(ref d) if d != did => return Err(crate::Error::Forbidden { action: "edit event" }),
        None => return Err(crate::Error::NotFound { entity: "event", id: id.to_string() }),
        _ => {}
    }

    sqlx::query(
        "UPDATE events SET title = $1, description = $2, kind = $3, location = $4, \
         online_url = $5, start_time = $6, end_time = $7, organizer = $8, \
         contact_email = $9, contact_url = $10, max_attendees = $11, updated_at = NOW() \
         WHERE id = $12",
    )
    .bind(&input.title)
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(&input.kind)
    .bind(&input.location)
    .bind(&input.online_url)
    .bind(&input.start_time)
    .bind(&input.end_time)
    .bind(&input.organizer)
    .bind(&input.contact_email)
    .bind(&input.contact_url)
    .bind(&input.max_attendees)
    .bind(id)
    .execute(pool)
    .await?;

    sqlx::query("DELETE FROM event_teaches WHERE event_id = $1").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM event_prereqs WHERE event_id = $1").bind(id).execute(pool).await?;
    for tag in &input.teaches {
        let _ = sqlx::query("INSERT INTO event_teaches (event_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id).bind(tag).execute(pool).await;
    }
    for tag in &input.prereqs {
        let _ = sqlx::query("INSERT INTO event_prereqs (event_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id).bind(tag).execute(pool).await;
    }

    get_event(pool, id).await
}

pub async fn cancel_event(pool: &PgPool, id: &str, did: &str) -> Result<()> {
    let result = sqlx::query("UPDATE events SET is_cancelled = true, updated_at = NOW() WHERE id = $1 AND did = $2")
        .bind(id).bind(did).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "event", id: id.to_string() });
    }
    Ok(())
}

pub async fn uncancel_event(pool: &PgPool, id: &str, did: &str) -> Result<()> {
    sqlx::query("UPDATE events SET is_cancelled = false, updated_at = NOW() WHERE id = $1 AND did = $2")
        .bind(id).bind(did).execute(pool).await?;
    Ok(())
}

pub async fn delete_event(pool: &PgPool, id: &str, did: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM events WHERE id = $1 AND did = $2")
        .bind(id).bind(did).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "event", id: id.to_string() });
    }
    Ok(())
}

pub async fn rsvp_event(pool: &PgPool, event_id: &str, did: &str, status: &str) -> Result<()> {
    if status != "going" && status != "interested" {
        return Err(crate::Error::BadRequest("status must be 'going' or 'interested'".into()));
    }

    // Check capacity for 'going'
    if status == "going" {
        let cap: Option<Option<i32>> = sqlx::query_scalar("SELECT max_attendees FROM events WHERE id = $1")
            .bind(event_id).fetch_optional(pool).await?;
        match cap {
            None => return Err(crate::Error::NotFound { entity: "event", id: event_id.to_string() }),
            Some(Some(max)) => {
                let going: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM event_rsvps WHERE event_id = $1 AND status = 'going' AND did != $2"
                ).bind(event_id).bind(did).fetch_one(pool).await?;
                if going >= max as i64 {
                    return Err(crate::Error::BadRequest("event is full".into()));
                }
            }
            _ => {}
        }
    }

    sqlx::query(
        "INSERT INTO event_rsvps (event_id, did, status) VALUES ($1, $2, $3) \
         ON CONFLICT (event_id, did) DO UPDATE SET status = $3"
    )
    .bind(event_id).bind(did).bind(status)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn cancel_rsvp(pool: &PgPool, event_id: &str, did: &str) -> Result<()> {
    sqlx::query("DELETE FROM event_rsvps WHERE event_id = $1 AND did = $2")
        .bind(event_id).bind(did).execute(pool).await?;
    Ok(())
}

pub async fn list_rsvps(pool: &PgPool, event_id: &str) -> Result<Vec<Rsvp>> {
    let rows = sqlx::query_as::<_, Rsvp>(
        "SELECT r.event_id, r.did, r.status, p.handle, p.display_name, r.created_at \
         FROM event_rsvps r LEFT JOIN profiles p ON r.did = p.did \
         WHERE r.event_id = $1 ORDER BY r.created_at"
    )
    .bind(event_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn my_rsvps(pool: &PgPool, did: &str) -> Result<Vec<Event>> {
    let sql = format!(
        "{EVENT_SELECT} WHERE e.id IN (SELECT event_id FROM event_rsvps WHERE did = $1) \
         ORDER BY e.start_time ASC"
    );
    let rows = sqlx::query_as::<_, Event>(&sql)
        .bind(did)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}
