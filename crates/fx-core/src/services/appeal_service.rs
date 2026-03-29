use sqlx::PgPool;

use crate::Result;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Appeal {
    pub id: String,
    pub did: String,
    pub kind: String,
    pub target_uri: Option<String>,
    pub reason: String,
    pub status: String,
    pub admin_response: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn create_appeal(
    pool: &PgPool,
    id: &str,
    did: &str,
    kind: &str,
    target_uri: Option<&str>,
    reason: &str,
) -> Result<Appeal> {
    let appeal = sqlx::query_as::<_, Appeal>(
        "INSERT INTO appeals (id, did, kind, target_uri, reason)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
    )
    .bind(id)
    .bind(did)
    .bind(kind)
    .bind(target_uri)
    .bind(reason)
    .fetch_one(pool)
    .await?;

    Ok(appeal)
}

pub async fn list_my_appeals(pool: &PgPool, did: &str) -> Result<Vec<Appeal>> {
    let rows = sqlx::query_as::<_, Appeal>(
        "SELECT * FROM appeals WHERE did = $1 ORDER BY created_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_pending_appeals(pool: &PgPool) -> Result<Vec<Appeal>> {
    let rows = sqlx::query_as::<_, Appeal>(
        "SELECT * FROM appeals WHERE status = 'pending' ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn resolve_appeal(
    pool: &PgPool,
    id: &str,
    status: &str,
    admin_response: Option<&str>,
) -> Result<Appeal> {
    let appeal = sqlx::query_as::<_, Appeal>(
        "UPDATE appeals SET status = $2, admin_response = $3, resolved_at = NOW()
         WHERE id = $1
         RETURNING *",
    )
    .bind(id)
    .bind(status)
    .bind(admin_response)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound {
        entity: "appeal",
        id: id.to_string(),
    })?;

    Ok(appeal)
}
