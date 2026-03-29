use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Member {
    pub author_did: String,
    pub member_did: String,
    pub created_at: DateTime<Utc>,
}

/// Add a member (grant access to all restricted content by author).
pub async fn add_member(pool: &PgPool, author_did: &str, member_did: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO user_members (author_did, member_did) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(author_did)
    .bind(member_did)
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove a member.
pub async fn remove_member(pool: &PgPool, author_did: &str, member_did: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM user_members WHERE author_did = $1 AND member_did = $2")
        .bind(author_did)
        .bind(member_did)
        .execute(pool)
        .await?;
    Ok(())
}

/// List all members of an author.
pub async fn list_members(pool: &PgPool, author_did: &str) -> crate::Result<Vec<Member>> {
    let rows = sqlx::query_as::<_, Member>(
        "SELECT * FROM user_members WHERE author_did = $1 ORDER BY created_at",
    )
    .bind(author_did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Check if a user is a member of an author.
pub async fn is_member(pool: &PgPool, author_did: &str, member_did: &str) -> crate::Result<bool> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_members WHERE author_did = $1 AND member_did = $2)",
    )
    .bind(author_did)
    .bind(member_did)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}
