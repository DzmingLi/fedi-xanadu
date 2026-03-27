use sqlx::PgPool;

use crate::Result;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BannedUser {
    pub did: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub banned_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ban_reason: Option<String>,
}

pub async fn ban_user(pool: &PgPool, did: &str, reason: Option<&str>) -> Result<()> {
    let result = sqlx::query(
        "UPDATE platform_users SET is_banned = true, banned_at = NOW(), ban_reason = $2 WHERE did = $1",
    )
    .bind(did)
    .bind(reason)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound {
            entity: "platform_user",
            id: did.to_string(),
        });
    }
    Ok(())
}

pub async fn unban_user(pool: &PgPool, did: &str) -> Result<()> {
    let result = sqlx::query(
        "UPDATE platform_users SET is_banned = false, banned_at = NULL, ban_reason = NULL WHERE did = $1",
    )
    .bind(did)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound {
            entity: "platform_user",
            id: did.to_string(),
        });
    }
    Ok(())
}

pub async fn is_user_banned(pool: &PgPool, did: &str) -> Result<bool> {
    let banned = sqlx::query_scalar::<_, bool>(
        "SELECT is_banned FROM platform_users WHERE did = $1",
    )
    .bind(did)
    .fetch_optional(pool)
    .await?;

    Ok(banned.unwrap_or(false))
}

pub async fn list_banned_users(pool: &PgPool) -> Result<Vec<BannedUser>> {
    let users = sqlx::query_as::<_, BannedUser>(
        "SELECT did, handle, display_name, banned_at, ban_reason
         FROM platform_users WHERE is_banned = true
         ORDER BY banned_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(users)
}
