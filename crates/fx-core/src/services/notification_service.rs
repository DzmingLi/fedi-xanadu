use sqlx::PgPool;

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct Notification {
    pub id: String,
    pub recipient_did: String,
    pub actor_did: String,
    pub actor_handle: Option<String>,
    pub kind: String,
    pub target_uri: Option<String>,
    pub target_title: Option<String>,
    pub context_id: Option<String>,
    pub read: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Create a notification. Silently skips if actor == recipient (don't notify yourself).
pub async fn create_notification(
    pool: &PgPool,
    id: &str,
    recipient_did: &str,
    actor_did: &str,
    kind: &str,
    target_uri: Option<&str>,
    context_id: Option<&str>,
) -> crate::Result<()> {
    if actor_did == recipient_did {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO notifications (id, recipient_did, actor_did, kind, target_uri, context_id)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(id)
    .bind(recipient_did)
    .bind(actor_did)
    .bind(kind)
    .bind(target_uri)
    .bind(context_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_notifications(
    pool: &PgPool,
    did: &str,
    limit: i64,
    offset: i64,
) -> crate::Result<Vec<Notification>> {
    let rows = sqlx::query_as::<_, Notification>(
        "SELECT n.id, n.recipient_did, n.actor_did,
                p.handle AS actor_handle,
                n.kind, n.target_uri,
                a.title AS target_title,
                n.context_id, n.read, n.created_at
         FROM notifications n
         LEFT JOIN profiles p ON p.did = n.actor_did
         LEFT JOIN articles a ON a.at_uri = n.target_uri
         WHERE n.recipient_did = $1
         ORDER BY n.created_at DESC
         LIMIT $2 OFFSET $3"
    )
    .bind(did)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn unread_count(pool: &PgPool, did: &str) -> crate::Result<i64> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM notifications WHERE recipient_did = $1 AND read = FALSE"
    )
    .bind(did)
    .fetch_one(pool)
    .await?;
    Ok(count.0)
}

pub async fn mark_read(pool: &PgPool, did: &str, id: &str) -> crate::Result<()> {
    sqlx::query(
        "UPDATE notifications SET read = TRUE WHERE id = $1 AND recipient_did = $2"
    )
    .bind(id)
    .bind(did)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_all_read(pool: &PgPool, did: &str) -> crate::Result<()> {
    sqlx::query(
        "UPDATE notifications SET read = TRUE WHERE recipient_did = $1 AND read = FALSE"
    )
    .bind(did)
    .execute(pool)
    .await?;
    Ok(())
}
