//! Reputation system.
//!
//! Reputation is computed from votes received on a user's content:
//!   - Article/lecture upvote:  +10
//!   - Answer upvote:           +10
//!   - Question upvote:         +10
//!   - Content downvote:         -2
//!   - Downvoting others:        -1  (cost to the voter, discourages frivolous downvotes)
//!   - Fork voted up:            +5
//!   - Comment votes:        no effect (prevents comment farming)
//!
//! The computed value is materialized in `profiles.reputation` and updated
//! incrementally when votes are cast.

use sqlx::PgPool;

use crate::Result;

/// Reputation points per event.
const CONTENT_UPVOTE: i64 = 10;
const CONTENT_DOWNVOTE: i64 = -2;
const DOWNVOTE_COST: i64 = -1;
const FORK_UPVOTE: i64 = 5;

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
    // 1. Votes received on user's articles/questions/answers (+10 / -2)
    let content_rep: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(\
            CASE \
                WHEN v.value > 0 THEN $2 \
                WHEN v.value < 0 THEN $3 \
                ELSE 0 \
            END\
         ), 0) \
         FROM votes v \
         JOIN articles a \
             ON article_uri(a.repo_uri, a.source_path) = v.target_uri \
         WHERE a.author_did = $1",
    )
    .bind(did)
    .bind(CONTENT_UPVOTE)
    .bind(CONTENT_DOWNVOTE)
    .fetch_one(pool)
    .await?;

    // 2. Cost of downvoting others (-1 per downvote cast by this user)
    let downvote_cost: i64 = sqlx::query_scalar(
        "SELECT COALESCE(COUNT(*), 0) FROM votes WHERE did = $1 AND value < 0",
    )
    .bind(did)
    .fetch_one(pool)
    .await?;

    // 3. Fork votes received (+5 per upvote on forks of user's articles)
    let fork_rep: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(\
            CASE WHEN v.value > 0 THEN $2 ELSE 0 END\
         ), 0) \
         FROM votes v \
         JOIN articles forked_a \
             ON article_uri(forked_a.repo_uri, forked_a.source_path) = v.target_uri \
         JOIN forks f \
             ON f.forked_repo_uri = forked_a.repo_uri \
            AND f.forked_source_path = forked_a.source_path \
         JOIN articles source_a \
             ON source_a.repo_uri = f.source_repo_uri \
            AND source_a.source_path = f.source_source_path \
         WHERE source_a.author_did = $1",
    )
    .bind(did)
    .bind(FORK_UPVOTE)
    .fetch_one(pool)
    .await?;

    let total = content_rep + (downvote_cost * DOWNVOTE_COST) + fork_rep;
    Ok(total.max(0))
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
/// Also recalculates the voter's reputation (downvote cost).
pub async fn update_for_content_vote(pool: &PgPool, target_uri: &str, voter_did: &str) -> Result<()> {
    // Update content author's reputation
    // target_uri is the synthetic article URI (nightboat://article/...).
    let author: Option<String> = sqlx::query_scalar(
        "SELECT author_did FROM articles \
         WHERE article_uri(repo_uri, source_path) = $1",
    )
        .bind(target_uri)
        .fetch_optional(pool)
        .await?;
    if let Some(did) = author {
        recalc_reputation(pool, &did).await?;
    }
    // Update voter's reputation (downvote cost affects voter)
    recalc_reputation(pool, voter_did).await?;
    Ok(())
}
