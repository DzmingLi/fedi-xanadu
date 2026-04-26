//! Term homeworks as a first-class entity. Replaces the JSONB-stuffed
//! `type='hw'` entries that used to live inside `term_sessions.resources`
//! — now they're rows with stable ids that questions, comments, and
//! future tag scopes can point at.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::util::tid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermHomework {
    pub id: String,
    pub term_id: String,
    pub session_id: Option<String>,
    pub label: String,
    pub url: Option<String>,
    pub description: String,
    pub position: i32,
    pub due_date: Option<NaiveDate>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateHomework {
    pub term_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub position: Option<i32>,
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateHomework {
    #[serde(default)]
    pub session_id: Option<Option<String>>,   // Some(None) clears, Some(Some(x)) sets
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub url: Option<Option<String>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub position: Option<i32>,
    #[serde(default)]
    pub due_date: Option<Option<NaiveDate>>,
}

pub async fn create_homework(
    pool: &PgPool,
    input: &CreateHomework,
    created_by: &str,
) -> crate::Result<TermHomework> {
    let id = format!("chw-{}", tid());
    sqlx::query(
        "INSERT INTO term_homeworks \
            (id, term_id, session_id, label, url, description, position, due_date, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&id)
    .bind(&input.term_id)
    .bind(&input.session_id)
    .bind(&input.label)
    .bind(&input.url)
    .bind(input.description.clone().unwrap_or_default())
    .bind(input.position.unwrap_or(0))
    .bind(input.due_date)
    .bind(created_by)
    .execute(pool)
    .await?;
    get_homework(pool, &id).await
}

pub async fn get_homework(pool: &PgPool, id: &str) -> crate::Result<TermHomework> {
    sqlx::query_as::<_, TermHomework>(
        "SELECT id, term_id, session_id, label, url, description, position, due_date, \
                created_by, created_at \
         FROM term_homeworks WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound { entity: "term_homework", id: id.to_string() })
}

pub async fn list_homeworks_by_term(pool: &PgPool, term_id: &str) -> crate::Result<Vec<TermHomework>> {
    Ok(sqlx::query_as::<_, TermHomework>(
        "SELECT id, term_id, session_id, label, url, description, position, due_date, \
                created_by, created_at \
         FROM term_homeworks WHERE term_id = $1 \
         ORDER BY position, created_at",
    )
    .bind(term_id)
    .fetch_all(pool)
    .await?)
}

pub async fn list_homeworks_by_session(pool: &PgPool, session_id: &str) -> crate::Result<Vec<TermHomework>> {
    Ok(sqlx::query_as::<_, TermHomework>(
        "SELECT id, term_id, session_id, label, url, description, position, due_date, \
                created_by, created_at \
         FROM term_homeworks WHERE session_id = $1 \
         ORDER BY position, created_at",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?)
}

pub async fn update_homework(
    pool: &PgPool,
    id: &str,
    input: &UpdateHomework,
) -> crate::Result<TermHomework> {
    // Fetch current row and overlay the Option<Option<...>> tri-state
    // fields so the caller can distinguish "leave alone" from "set null".
    let cur = get_homework(pool, id).await?;
    let session_id = match &input.session_id { Some(v) => v.clone(), None => cur.session_id.clone() };
    let label = input.label.clone().unwrap_or(cur.label.clone());
    let url = match &input.url { Some(v) => v.clone(), None => cur.url.clone() };
    let description = input.description.clone().unwrap_or(cur.description.clone());
    let position = input.position.unwrap_or(cur.position);
    let due_date = match &input.due_date { Some(v) => *v, None => cur.due_date };

    sqlx::query(
        "UPDATE term_homeworks \
            SET session_id = $2, label = $3, url = $4, description = $5, position = $6, due_date = $7 \
          WHERE id = $1",
    )
    .bind(id)
    .bind(&session_id)
    .bind(&label)
    .bind(&url)
    .bind(&description)
    .bind(position)
    .bind(due_date)
    .execute(pool)
    .await?;
    get_homework(pool, id).await
}

pub async fn delete_homework(pool: &PgPool, id: &str) -> crate::Result<()> {
    let res = sqlx::query("DELETE FROM term_homeworks WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "term_homework", id: id.to_string() });
    }
    Ok(())
}
