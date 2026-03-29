use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct LearnedMark {
    pub did: String,
    pub article_uri: String,
    pub learned_at: DateTime<Utc>,
}

/// Mark an article as learned by a user.
pub async fn mark_learned(pool: &PgPool, did: &str, article_uri: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO learned_marks (did, article_uri) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(did)
    .bind(article_uri)
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove the learned mark.
pub async fn unmark_learned(pool: &PgPool, did: &str, article_uri: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM learned_marks WHERE did = $1 AND article_uri = $2")
        .bind(did)
        .bind(article_uri)
        .execute(pool)
        .await?;
    Ok(())
}

/// Check if a user has learned a specific article.
pub async fn is_learned(pool: &PgPool, did: &str, article_uri: &str) -> crate::Result<bool> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM learned_marks WHERE did = $1 AND article_uri = $2)",
    )
    .bind(did)
    .bind(article_uri)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// List all learned articles for a user.
pub async fn list_learned(pool: &PgPool, did: &str) -> crate::Result<Vec<LearnedMark>> {
    let rows = sqlx::query_as::<_, LearnedMark>(
        "SELECT did, article_uri, learned_at FROM learned_marks WHERE did = $1 ORDER BY learned_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Derive user skills from learned marks: returns tag_ids the user has mastered
/// by reading articles that teach those tags.
pub async fn derived_skills(pool: &PgPool, did: &str) -> crate::Result<Vec<String>> {
    let tags = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT ct.tag_id \
         FROM learned_marks lm \
         JOIN content_teaches ct ON ct.content_uri = lm.article_uri \
         WHERE lm.did = $1",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(tags)
}
