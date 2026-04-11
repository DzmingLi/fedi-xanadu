use sqlx::PgPool;

// --- Series collaborators ---

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Collaborator {
    pub series_id: String,
    pub user_did: String,
    pub channel_name: String,
    pub role: String,
    pub invited_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_collaborators(pool: &PgPool, series_id: &str) -> crate::Result<Vec<Collaborator>> {
    let rows = sqlx::query_as::<_, Collaborator>(
        "SELECT series_id, user_did, channel_name, role, invited_by, created_at \
         FROM series_collaborators WHERE series_id = $1 ORDER BY created_at",
    )
    .bind(series_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn add_collaborator(
    pool: &PgPool,
    series_id: &str,
    user_did: &str,
    role: &str,
    channel_name: &str,
    invited_by: &str,
) -> crate::Result<Collaborator> {
    let row = sqlx::query_as::<_, Collaborator>(
        "INSERT INTO series_collaborators (series_id, user_did, channel_name, role, invited_by) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (series_id, user_did) DO UPDATE SET role = $4 \
         RETURNING series_id, user_did, channel_name, role, invited_by, created_at",
    )
    .bind(series_id)
    .bind(user_did)
    .bind(channel_name)
    .bind(role)
    .bind(invited_by)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn remove_collaborator(pool: &PgPool, series_id: &str, user_did: &str) -> crate::Result<bool> {
    let result = sqlx::query(
        "DELETE FROM series_collaborators WHERE series_id = $1 AND user_did = $2 AND role != 'owner'",
    )
    .bind(series_id)
    .bind(user_did)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_collaborator(pool: &PgPool, series_id: &str, user_did: &str) -> crate::Result<Option<Collaborator>> {
    let row = sqlx::query_as::<_, Collaborator>(
        "SELECT series_id, user_did, channel_name, role, invited_by, created_at \
         FROM series_collaborators WHERE series_id = $1 AND user_did = $2",
    )
    .bind(series_id)
    .bind(user_did)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn register_owner(pool: &PgPool, series_id: &str, owner_did: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO series_collaborators (series_id, user_did, channel_name, role) \
         VALUES ($1, $2, 'main', 'owner') ON CONFLICT DO NOTHING",
    )
    .bind(series_id)
    .bind(owner_did)
    .execute(pool)
    .await?;
    Ok(())
}

// --- Article collaborators ---

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ArticleCollaborator {
    pub article_uri: String,
    pub user_did: String,
    pub channel_name: String,
    pub role: String,
    pub invited_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_article_collaborators(pool: &PgPool, article_uri: &str) -> crate::Result<Vec<ArticleCollaborator>> {
    let rows = sqlx::query_as::<_, ArticleCollaborator>(
        "SELECT article_uri, user_did, channel_name, role, invited_by, created_at \
         FROM article_collaborators WHERE article_uri = $1 ORDER BY created_at",
    )
    .bind(article_uri)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn add_article_collaborator(
    pool: &PgPool,
    article_uri: &str,
    user_did: &str,
    role: &str,
    channel_name: &str,
    invited_by: &str,
) -> crate::Result<ArticleCollaborator> {
    let row = sqlx::query_as::<_, ArticleCollaborator>(
        "INSERT INTO article_collaborators (article_uri, user_did, channel_name, role, invited_by) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (article_uri, user_did) DO UPDATE SET role = $4 \
         RETURNING article_uri, user_did, channel_name, role, invited_by, created_at",
    )
    .bind(article_uri)
    .bind(user_did)
    .bind(channel_name)
    .bind(role)
    .bind(invited_by)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn remove_article_collaborator(pool: &PgPool, article_uri: &str, user_did: &str) -> crate::Result<bool> {
    let result = sqlx::query(
        "DELETE FROM article_collaborators WHERE article_uri = $1 AND user_did = $2 AND role != 'owner'",
    )
    .bind(article_uri)
    .bind(user_did)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_article_collaborator(pool: &PgPool, article_uri: &str, user_did: &str) -> crate::Result<Option<ArticleCollaborator>> {
    let row = sqlx::query_as::<_, ArticleCollaborator>(
        "SELECT article_uri, user_did, channel_name, role, invited_by, created_at \
         FROM article_collaborators WHERE article_uri = $1 AND user_did = $2",
    )
    .bind(article_uri)
    .bind(user_did)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn register_article_owner(pool: &PgPool, article_uri: &str, owner_did: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO article_collaborators (article_uri, user_did, channel_name, role) \
         VALUES ($1, $2, 'main', 'owner') ON CONFLICT DO NOTHING",
    )
    .bind(article_uri)
    .bind(owner_did)
    .execute(pool)
    .await?;
    Ok(())
}
