use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookmarkWithTitle {
    /// Synthetic article URI (`nightboat://article/{repo_uri}/{source_path}`).
    pub article_uri: String,
    pub folder_path: String,
    pub created_at: DateTime<Utc>,
    pub title: String,
    pub summary: String,
    pub kind: String,
    pub question_uri: Option<String>,
}

pub async fn list_bookmarks(pool: &PgPool, did: &str) -> crate::Result<Vec<BookmarkWithTitle>> {
    let rows = sqlx::query_as::<_, BookmarkWithTitle>(
        "SELECT article_uri(b.repo_uri, b.source_path) AS article_uri, \
                b.folder_path, b.created_at, l.title, l.summary, \
                a.kind::TEXT AS kind, \
                CASE WHEN a.question_repo_uri IS NULL THEN NULL \
                     ELSE article_uri(a.question_repo_uri, a.question_source_path) \
                END AS question_uri \
         FROM user_bookmarks b \
         JOIN articles a \
             ON a.repo_uri = b.repo_uri AND a.source_path = b.source_path \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
            AND l.file_path = a.source_path \
         WHERE b.did = $1 \
         ORDER BY b.folder_path, b.created_at",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// `article_uri` is a localization at_uri; resolved to composite key on insert.
pub async fn add_bookmark(
    pool: &PgPool,
    did: &str,
    article_uri: &str,
    folder_path: &str,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO user_bookmarks (did, repo_uri, source_path, folder_path) \
         SELECT $1, repo_uri, source_path, $3 \
         FROM article_localizations WHERE at_uri = $2 \
         ON CONFLICT (did, repo_uri, source_path) DO UPDATE SET folder_path = EXCLUDED.folder_path",
    )
    .bind(did)
    .bind(article_uri)
    .bind(folder_path)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_bookmark(
    pool: &PgPool,
    did: &str,
    article_uri: &str,
) -> crate::Result<()> {
    sqlx::query(
        "DELETE FROM user_bookmarks \
         WHERE did = $1 \
         AND (repo_uri, source_path) IN \
             (SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $2)",
    )
        .bind(did)
        .bind(article_uri)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn move_bookmark(
    pool: &PgPool,
    did: &str,
    article_uri: &str,
    folder_path: &str,
) -> crate::Result<()> {
    sqlx::query(
        "UPDATE user_bookmarks SET folder_path = $1 \
         WHERE did = $2 \
         AND (repo_uri, source_path) IN \
             (SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $3)",
    )
        .bind(folder_path)
        .bind(did)
        .bind(article_uri)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_bookmark_folders(pool: &PgPool, did: &str) -> crate::Result<Vec<String>> {
    let folders: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT folder_path FROM user_bookmarks WHERE did = $1 ORDER BY folder_path",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(folders)
}
