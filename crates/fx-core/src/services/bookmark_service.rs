use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookmarkWithTitle {
    pub article_uri: String,
    pub folder_path: String,
    pub created_at: DateTime<Utc>,
    pub title: String,
    pub summary: String,
    /// Content kind of the bookmarked record (article / question / answer / thought).
    /// Clients use this to route to the correct canonical page (e.g. `/question`
    /// for aggregated answer threads).
    pub kind: String,
    pub question_uri: Option<String>,
}

pub async fn list_bookmarks(pool: &PgPool, did: &str) -> crate::Result<Vec<BookmarkWithTitle>> {
    let rows = sqlx::query_as::<_, BookmarkWithTitle>(
        "SELECT b.article_uri, b.folder_path, b.created_at, a.title, a.summary, \
                a.kind::TEXT AS kind, a.question_uri \
         FROM user_bookmarks b \
         JOIN articles a ON a.at_uri = b.article_uri \
         WHERE b.did = $1 \
         ORDER BY b.folder_path, b.created_at",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn add_bookmark(
    pool: &PgPool,
    did: &str,
    article_uri: &str,
    folder_path: &str,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO user_bookmarks (did, article_uri, folder_path) VALUES ($1, $2, $3) \
         ON CONFLICT (did, article_uri) DO UPDATE SET folder_path = EXCLUDED.folder_path",
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
    sqlx::query("DELETE FROM user_bookmarks WHERE did = $1 AND article_uri = $2")
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
    sqlx::query("UPDATE user_bookmarks SET folder_path = $1 WHERE did = $2 AND article_uri = $3")
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
