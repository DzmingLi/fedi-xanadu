use sqlx::PgPool;

use crate::models::ArticleAuthor;

/// Add an author to an article (status = 'pending' unless self-adding).
pub async fn add_author(
    pool: &PgPool,
    article_uri: &str,
    author_did: &str,
    added_by: &str,
    position: Option<i16>,
) -> crate::Result<()> {
    let status = if author_did == added_by { "verified" } else { "pending" };
    let verified_at: Option<chrono::DateTime<chrono::Utc>> = if author_did == added_by {
        Some(chrono::Utc::now())
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO article_authors (article_uri, author_did, position, status, added_by, verified_at) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (article_uri, author_did) WHERE author_did IS NOT NULL DO NOTHING",
    )
    .bind(article_uri)
    .bind(author_did)
    .bind(position)
    .bind(status)
    .bind(added_by)
    .bind(verified_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Add a non-user author by name only (no DID).
pub async fn add_author_by_name(
    pool: &PgPool,
    article_uri: &str,
    author_name: &str,
    added_by: &str,
    position: Option<i16>,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO article_authors (article_uri, author_name, position, status, added_by) \
         VALUES ($1, $2, $3, 'verified', $4)",
    )
    .bind(article_uri)
    .bind(author_name)
    .bind(position)
    .bind(added_by)
    .execute(pool)
    .await?;
    Ok(())
}

/// Verify authorship (set status to 'verified' and record the PDS authorship URI).
pub async fn verify_authorship(
    pool: &PgPool,
    article_uri: &str,
    author_did: &str,
    authorship_uri: Option<&str>,
) -> crate::Result<bool> {
    let result = sqlx::query(
        "UPDATE article_authors SET status = 'verified', authorship_uri = $3, verified_at = NOW() \
         WHERE article_uri = $1 AND author_did = $2 AND status = 'pending'",
    )
    .bind(article_uri)
    .bind(author_did)
    .bind(authorship_uri)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Reject authorship (author declares "this is not me").
pub async fn reject_authorship(
    pool: &PgPool,
    article_uri: &str,
    author_did: &str,
) -> crate::Result<bool> {
    let result = sqlx::query(
        "UPDATE article_authors SET status = 'rejected', verified_at = NOW() \
         WHERE article_uri = $1 AND author_did = $2 AND status != 'rejected'",
    )
    .bind(article_uri)
    .bind(author_did)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// List non-rejected authors for an article, ordered by position (nulls last).
pub async fn list_authors(
    pool: &PgPool,
    article_uri: &str,
) -> crate::Result<Vec<ArticleAuthor>> {
    let rows = sqlx::query_as::<_, ArticleAuthor>(
        "SELECT aa.author_did, aa.author_name, p.handle AS author_handle, \
                p.display_name AS author_display_name, p.avatar_url AS author_avatar, \
                COALESCE(p.reputation, 0) AS author_reputation, \
                aa.position, aa.status, aa.authorship_uri \
         FROM article_authors aa \
         LEFT JOIN profiles p ON aa.author_did = p.did \
         WHERE aa.article_uri = $1 AND aa.status != 'rejected' \
         ORDER BY aa.position NULLS LAST, aa.created_at",
    )
    .bind(article_uri)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List authors for multiple articles in a single query (for list endpoints).
pub async fn list_authors_batch(
    pool: &PgPool,
    article_uris: &[String],
) -> crate::Result<Vec<(String, ArticleAuthor)>> {
    if article_uris.is_empty() {
        return Ok(vec![]);
    }

    let rows: Vec<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, i32, Option<i16>, String, Option<String>)> =
        sqlx::query_as(
            "SELECT aa.article_uri, aa.author_did, aa.author_name, p.handle, \
                    p.display_name, p.avatar_url, \
                    COALESCE(p.reputation, 0), \
                    aa.position, aa.status, aa.authorship_uri \
             FROM article_authors aa \
             LEFT JOIN profiles p ON aa.author_did = p.did \
             WHERE aa.article_uri = ANY($1) AND aa.status != 'rejected' \
             ORDER BY aa.position NULLS LAST, aa.created_at",
        )
        .bind(article_uris)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|(uri, did, name, handle, display_name, avatar, rep, pos, status, auri)| {
            (
                uri,
                ArticleAuthor {
                    author_did: did,
                    author_name: name,
                    author_handle: handle,
                    author_display_name: display_name,
                    author_avatar: avatar,
                    author_reputation: rep,
                    position: pos,
                    status,
                    authorship_uri: auri,
                },
            )
        })
        .collect())
}

/// List pending authorships for a user (for notifications / dashboard).
pub async fn list_pending_for_user(
    pool: &PgPool,
    author_did: &str,
) -> crate::Result<Vec<(String, String)>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT aa.article_uri, a.title \
         FROM article_authors aa \
         JOIN articles a ON aa.article_uri = a.at_uri \
         WHERE aa.author_did = $1 AND aa.status = 'pending' \
         ORDER BY aa.created_at DESC",
    )
    .bind(author_did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Remove an author entry (only the article publisher or the author themselves can do this).
pub async fn remove_author(
    pool: &PgPool,
    article_uri: &str,
    author_did: &str,
) -> crate::Result<bool> {
    let result = sqlx::query(
        "DELETE FROM article_authors WHERE article_uri = $1 AND author_did = $2",
    )
    .bind(article_uri)
    .bind(author_did)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
