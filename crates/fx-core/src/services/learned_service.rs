use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct LearnedMark {
    pub did: String,
    /// Synthetic article URI (`nightboat://article/{repo_uri}/{source_path}`).
    pub article_uri: String,
    pub learned_at: DateTime<Utc>,
}

/// Mark an article as learned by a user. Accepts at_uri or chapter URI.
pub async fn mark_learned(pool: &PgPool, did: &str, article_uri: &str) -> crate::Result<()> {
    let Some((repo_uri, source_path)) = super::series_service::resolve_to_repo_path(pool, article_uri).await? else {
        return Err(crate::Error::NotFound { entity: "article", id: article_uri.to_string() });
    };
    sqlx::query(
        "INSERT INTO learned_marks (did, repo_uri, source_path) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(did)
    .bind(&repo_uri)
    .bind(&source_path)
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove the learned mark.
pub async fn unmark_learned(pool: &PgPool, did: &str, article_uri: &str) -> crate::Result<()> {
    let Some((repo_uri, source_path)) = super::series_service::resolve_to_repo_path(pool, article_uri).await? else {
        return Ok(());
    };
    sqlx::query("DELETE FROM learned_marks WHERE did = $1 AND repo_uri = $2 AND source_path = $3")
        .bind(did)
        .bind(&repo_uri)
        .bind(&source_path)
        .execute(pool)
        .await?;
    Ok(())
}

/// Check if a user has learned a specific article.
pub async fn is_learned(pool: &PgPool, did: &str, article_uri: &str) -> crate::Result<bool> {
    let Some((repo_uri, source_path)) = super::series_service::resolve_to_repo_path(pool, article_uri).await? else {
        return Ok(false);
    };
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM learned_marks WHERE did = $1 AND repo_uri = $2 AND source_path = $3)",
    )
    .bind(did)
    .bind(&repo_uri)
    .bind(&source_path)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// List all learned articles for a user.
pub async fn list_learned(pool: &PgPool, did: &str) -> crate::Result<Vec<LearnedMark>> {
    let rows = sqlx::query_as::<_, LearnedMark>(
        "SELECT did, article_uri(repo_uri, source_path) AS article_uri, learned_at \
         FROM learned_marks WHERE did = $1 ORDER BY learned_at DESC",
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
         JOIN content_teaches ct ON ct.content_uri = article_uri(lm.repo_uri, lm.source_path) \
         WHERE lm.did = $1",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(tags)
}
