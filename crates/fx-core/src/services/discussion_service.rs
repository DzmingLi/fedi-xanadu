use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Discussion {
    pub id: String,
    pub target_uri: String,
    pub source_uri: String,
    pub author_did: String,
    pub title: String,
    pub body: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct DiscussionChange {
    pub id: i32,
    pub discussion_id: String,
    pub change_hash: String,
    pub added_by: String,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub applied: bool,
    pub applied_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscussionDetail {
    pub discussion: Discussion,
    pub changes: Vec<DiscussionChange>,
}

pub async fn create_discussion(
    pool: &PgPool,
    id: &str,
    target_uri: &str,
    source_uri: &str,
    author_did: &str,
    title: &str,
    body: Option<&str>,
    change_hashes: &[String],
) -> crate::Result<Discussion> {
    let mut tx = pool.begin().await?;

    let disc = sqlx::query_as::<_, Discussion>(
        "INSERT INTO discussions (id, target_uri, source_uri, author_did, title, body) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, target_uri, source_uri, author_did, title, body, status, created_at, updated_at",
    )
    .bind(id)
    .bind(target_uri)
    .bind(source_uri)
    .bind(author_did)
    .bind(title)
    .bind(body)
    .fetch_one(&mut *tx)
    .await?;

    for hash in change_hashes {
        sqlx::query(
            "INSERT INTO discussion_changes (discussion_id, change_hash, added_by) VALUES ($1, $2, $3)",
        )
        .bind(id)
        .bind(hash)
        .bind(author_did)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(disc)
}

pub async fn list_discussions(pool: &PgPool, target_uri: &str) -> crate::Result<Vec<Discussion>> {
    let rows = sqlx::query_as::<_, Discussion>(
        "SELECT id, target_uri, source_uri, author_did, title, body, status, created_at, updated_at \
         FROM discussions WHERE target_uri = $1 ORDER BY created_at DESC",
    )
    .bind(target_uri)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_discussion(pool: &PgPool, id: &str) -> crate::Result<DiscussionDetail> {
    let disc = sqlx::query_as::<_, Discussion>(
        "SELECT id, target_uri, source_uri, author_did, title, body, status, created_at, updated_at \
         FROM discussions WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    let changes = sqlx::query_as::<_, DiscussionChange>(
        "SELECT id, discussion_id, change_hash, added_by, added_at, applied, applied_at \
         FROM discussion_changes WHERE discussion_id = $1 ORDER BY added_at",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(DiscussionDetail { discussion: disc, changes })
}

pub async fn update_status(pool: &PgPool, id: &str, status: &str) -> crate::Result<()> {
    sqlx::query("UPDATE discussions SET status = $1, updated_at = NOW() WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_change_applied(pool: &PgPool, discussion_id: &str, change_hash: &str) -> crate::Result<()> {
    sqlx::query(
        "UPDATE discussion_changes SET applied = TRUE, applied_at = NOW() \
         WHERE discussion_id = $1 AND change_hash = $2",
    )
    .bind(discussion_id)
    .bind(change_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn all_changes_applied(pool: &PgPool, discussion_id: &str) -> crate::Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM discussion_changes WHERE discussion_id = $1 AND applied = FALSE",
    )
    .bind(discussion_id)
    .fetch_one(pool)
    .await?;
    Ok(count == 0)
}
