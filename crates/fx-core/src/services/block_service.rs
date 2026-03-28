use sqlx::PgPool;

/// Block a user. The blocker will no longer see their content.
pub async fn block_user(pool: &PgPool, did: &str, blocked_did: &str) -> crate::Result<()> {
    if did == blocked_did {
        return Err(crate::Error::BadRequest("cannot block yourself".into()));
    }
    sqlx::query(
        "INSERT INTO user_blocks (did, blocked_did) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(did)
    .bind(blocked_did)
    .execute(pool)
    .await?;
    Ok(())
}

/// Unblock a user.
pub async fn unblock_user(pool: &PgPool, did: &str, blocked_did: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM user_blocks WHERE did = $1 AND blocked_did = $2")
        .bind(did)
        .bind(blocked_did)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get the set of DIDs blocked by a user.
pub async fn list_blocked_dids(pool: &PgPool, did: &str) -> crate::Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT blocked_did FROM user_blocks WHERE did = $1 ORDER BY created_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BlockedUser {
    pub blocked_did: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// List blocked users with profile info.
pub async fn list_blocked_users(pool: &PgPool, did: &str) -> crate::Result<Vec<BlockedUser>> {
    let rows = sqlx::query_as::<_, BlockedUser>(
        "SELECT b.blocked_did, p.handle, p.display_name, p.avatar_url, b.created_at \
         FROM user_blocks b \
         LEFT JOIN profiles p ON b.blocked_did = p.did \
         WHERE b.did = $1 \
         ORDER BY b.created_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Check if `did` has blocked `target_did`.
pub async fn is_blocked(pool: &PgPool, did: &str, target_did: &str) -> crate::Result<bool> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_blocks WHERE did = $1 AND blocked_did = $2)",
    )
    .bind(did)
    .bind(target_did)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}
