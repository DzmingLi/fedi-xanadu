//! Reputation system.
//!
//! Reputation is computed from votes received on a user's content:
//!   - Article upvote:   +10
//!   - Answer upvote:    +15
//!   - Question upvote:   +5
//!   - Any downvote:      -2
//!   - Comment upvote:    +2
//!   - Comment downvote:  -1
//!
//! The computed value is materialized in `profiles.reputation` and updated
//! incrementally when votes are cast.

use sqlx::PgPool;

use crate::Result;

/// Reputation points per event.
const ARTICLE_UPVOTE: i64 = 10;
const ANSWER_UPVOTE: i64 = 15;
const QUESTION_UPVOTE: i64 = 5;
const CONTENT_DOWNVOTE: i64 = -2;
const COMMENT_UPVOTE: i64 = 2;
const COMMENT_DOWNVOTE: i64 = -1;

/// Recalculate and store reputation for a single user from scratch.
pub async fn recalc_reputation(pool: &PgPool, did: &str) -> Result<i64> {
    let rep = compute_reputation(pool, did).await?;
    sqlx::query("UPDATE profiles SET reputation = $1 WHERE did = $2")
        .bind(rep)
        .bind(did)
        .execute(pool)
        .await?;
    Ok(rep)
}

/// Compute reputation from the vote tables (does not write).
async fn compute_reputation(pool: &PgPool, did: &str) -> Result<i64> {
    // Content votes (articles, questions, answers)
    let content_rep: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(\
            CASE \
                WHEN v.value > 0 AND a.kind = 'answer' THEN $2 \
                WHEN v.value > 0 AND a.kind = 'question' THEN $3 \
                WHEN v.value > 0 THEN $4 \
                WHEN v.value < 0 THEN $5 \
                ELSE 0 \
            END\
         ), 0) \
         FROM votes v \
         JOIN articles a ON a.at_uri = v.target_uri \
         WHERE a.did = $1",
    )
    .bind(did)
    .bind(ANSWER_UPVOTE)
    .bind(QUESTION_UPVOTE)
    .bind(ARTICLE_UPVOTE)
    .bind(CONTENT_DOWNVOTE)
    .fetch_one(pool)
    .await?;

    // Comment votes
    let comment_rep: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(\
            CASE \
                WHEN cv.value > 0 THEN $2 \
                WHEN cv.value < 0 THEN $3 \
                ELSE 0 \
            END\
         ), 0) \
         FROM comment_votes cv \
         JOIN comments c ON c.id = cv.comment_id \
         WHERE c.did = $1",
    )
    .bind(did)
    .bind(COMMENT_UPVOTE)
    .bind(COMMENT_DOWNVOTE)
    .fetch_one(pool)
    .await?;

    Ok((content_rep + comment_rep).max(0))
}

/// Recalculate reputation for ALL users. Used as a background safety net.
pub async fn recalc_all(pool: &PgPool) -> Result<u64> {
    let dids: Vec<String> =
        sqlx::query_scalar("SELECT did FROM profiles")
            .fetch_all(pool)
            .await?;

    let mut count = 0u64;
    for did in &dids {
        let _ = recalc_reputation(pool, did).await;
        count += 1;
    }
    Ok(count)
}

/// Look up the author DID of a vote target (article) and recalculate their reputation.
pub async fn update_for_content_vote(pool: &PgPool, target_uri: &str) -> Result<()> {
    let author: Option<String> =
        sqlx::query_scalar("SELECT did FROM articles WHERE at_uri = $1")
            .bind(target_uri)
            .fetch_optional(pool)
            .await?;
    if let Some(did) = author {
        recalc_reputation(pool, &did).await?;
    }
    Ok(())
}

/// Look up the author DID of a comment and recalculate their reputation.
pub async fn update_for_comment_vote(pool: &PgPool, comment_id: &str) -> Result<()> {
    let author: Option<String> =
        sqlx::query_scalar("SELECT did FROM comments WHERE id = $1")
            .bind(comment_id)
            .fetch_optional(pool)
            .await?;
    if let Some(did) = author {
        recalc_reputation(pool, &did).await?;
    }
    Ok(())
}
