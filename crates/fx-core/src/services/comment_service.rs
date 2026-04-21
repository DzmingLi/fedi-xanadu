use sqlx::PgPool;

use crate::models::Comment;
use crate::Result;
use crate::Error;

const COMMENT_SELECT: &str = "\
    SELECT c.id, c.content_uri, c.did, p.handle AS author_handle, c.parent_id, c.title, c.body, c.quote_text, \
    c.section_ref, \
    COALESCE((SELECT SUM(value) FROM comment_votes WHERE comment_id = c.id), 0) AS vote_score, \
    c.created_at, c.updated_at \
    FROM comments c LEFT JOIN profiles p ON c.did = p.did";

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct MyCommentVote {
    pub comment_id: String,
    pub value: i32,
}

pub async fn list_comments(pool: &PgPool, content_uri: &str, section_ref: Option<&str>, limit: i64) -> Result<Vec<Comment>> {
    let comments = if let Some(sec) = section_ref {
        sqlx::query_as::<_, Comment>(
            &format!("{COMMENT_SELECT} WHERE c.content_uri = $1 AND c.section_ref = $2 ORDER BY c.created_at ASC LIMIT $3"),
        )
        .bind(content_uri)
        .bind(sec)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Comment>(
            &format!("{COMMENT_SELECT} WHERE c.content_uri = $1 ORDER BY c.created_at ASC LIMIT $2"),
        )
        .bind(content_uri)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    Ok(comments)
}

/// List top-level comments (no replies) for a given content, ordered by
/// vote score then creation time, with pagination. Used by course/book
/// discussion summaries + full listing pages.
pub async fn list_top_comments(
    pool: &PgPool,
    content_uri: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Comment>> {
    let comments = sqlx::query_as::<_, Comment>(
        &format!(
            "{COMMENT_SELECT} WHERE c.content_uri = $1 AND c.parent_id IS NULL \
             ORDER BY vote_score DESC, c.created_at DESC LIMIT $2 OFFSET $3"
        ),
    )
    .bind(content_uri)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(comments)
}

/// Count top-level comments for a given content.
pub async fn count_top_comments(pool: &PgPool, content_uri: &str) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM comments WHERE content_uri = $1 AND parent_id IS NULL",
    )
    .bind(content_uri)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn create_comment(
    pool: &PgPool,
    id: &str,
    content_uri: &str,
    did: &str,
    title: Option<&str>,
    body: &str,
    parent_id: Option<&str>,
    quote_text: Option<&str>,
    section_ref: Option<&str>,
) -> Result<Comment> {
    sqlx::query(
        "INSERT INTO comments (id, content_uri, did, title, body, parent_id, quote_text, section_ref) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(content_uri)
    .bind(did)
    .bind(title)
    .bind(body)
    .bind(parent_id)
    .bind(quote_text)
    .bind(section_ref)
    .execute(pool)
    .await?;

    let comment = sqlx::query_as::<_, Comment>(
        &format!("{COMMENT_SELECT} WHERE c.id = $1"),
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(comment)
}

pub async fn update_comment(pool: &PgPool, id: &str, body: &str) -> Result<Comment> {
    sqlx::query("UPDATE comments SET body = $1, updated_at = NOW() WHERE id = $2")
        .bind(body)
        .bind(id)
        .execute(pool)
        .await?;

    let comment = sqlx::query_as::<_, Comment>(
        &format!("{COMMENT_SELECT} WHERE c.id = $1"),
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(comment)
}

/// Delete a comment. ON DELETE CASCADE handles child comments and comment_votes.
pub async fn delete_comment(pool: &PgPool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Returns `(comment_did, content_author_did)` for authorization checks.
///
/// `content_uri` is polymorphic: it may point at an article (`at://...`), a
/// book (`book:<id>`), a chapter (`chapter:<id>` → falls back to the book's
/// creator), or a book series (`book_series:<id>`). The first matching
/// content-author lookup wins.
pub async fn get_comment_owner(pool: &PgPool, id: &str) -> Result<(String, String)> {
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT c.did, \
                COALESCE( \
                    (SELECT a.did FROM articles a WHERE a.at_uri = c.content_uri), \
                    (SELECT b.created_by FROM books b WHERE 'book:' || b.id = c.content_uri), \
                    (SELECT bk.created_by FROM book_chapters bc \
                                          JOIN books bk ON bk.id = bc.book_id \
                      WHERE 'chapter:' || bc.id = c.content_uri), \
                    (SELECT bs.created_by FROM book_series bs WHERE 'book_series:' || bs.id = c.content_uri) \
                ) \
           FROM comments c \
          WHERE c.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((commenter, Some(author))) => Ok((commenter, author)),
        Some((_, None)) => Err(Error::NotFound {
            entity: "comment target",
            id: id.to_string(),
        }),
        None => Err(Error::NotFound {
            entity: "comment",
            id: id.to_string(),
        }),
    }
}

/// Validates that `parent_id` exists and belongs to the same content.
pub async fn verify_parent_comment(
    pool: &PgPool,
    parent_id: &str,
    content_uri: &str,
) -> Result<()> {
    let parent_uri: Option<String> =
        sqlx::query_scalar("SELECT content_uri FROM comments WHERE id = $1")
            .bind(parent_id)
            .fetch_optional(pool)
            .await?;

    match parent_uri {
        Some(uri) if uri == content_uri => Ok(()),
        Some(_) => Err(Error::Conflict {
            message: "parent comment belongs to different content".into(),
        }),
        None => Err(Error::NotFound {
            entity: "parent comment",
            id: parent_id.to_string(),
        }),
    }
}

/// Upserts a vote (value clamped to -1..1, 0 removes the vote).
/// Returns the new aggregate score for the comment, computed atomically in the same transaction.
pub async fn vote_comment(pool: &PgPool, comment_id: &str, did: &str, value: i32) -> Result<i64> {
    let value = value.clamp(-1, 1);

    let mut tx = pool.begin().await?;

    if value == 0 {
        sqlx::query("DELETE FROM comment_votes WHERE comment_id = $1 AND did = $2")
            .bind(comment_id)
            .bind(did)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query(
            "INSERT INTO comment_votes (comment_id, did, value) VALUES ($1, $2, $3) \
             ON CONFLICT(comment_id, did) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(comment_id)
        .bind(did)
        .bind(value)
        .execute(&mut *tx)
        .await?;
    }

    let score: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(value), 0) FROM comment_votes WHERE comment_id = $1",
    )
    .bind(comment_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(score)
}

/// Returns the current user's votes on all comments for a given content.
pub async fn get_my_comment_votes(
    pool: &PgPool,
    did: &str,
    content_uri: &str,
) -> Result<Vec<MyCommentVote>> {
    let votes = sqlx::query_as::<_, MyCommentVote>(
        "SELECT cv.comment_id, cv.value FROM comment_votes cv \
         JOIN comments c ON c.id = cv.comment_id \
         WHERE cv.did = $1 AND c.content_uri = $2",
    )
    .bind(did)
    .bind(content_uri)
    .fetch_all(pool)
    .await?;

    Ok(votes)
}
