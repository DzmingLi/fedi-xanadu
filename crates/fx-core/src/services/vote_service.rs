use sqlx::PgPool;

use crate::Result;

/// Normalize an article-side `target_uri` to the synthetic article URI form
/// used as the canonical key in `votes.target_uri`. Accepts at_uri and
/// `nightboat-chapter://` URIs; non-article targets pass through unchanged
/// (e.g. comment/skill-tree URIs vote against their own at_uri).
async fn normalize_vote_target(pool: &PgPool, target_uri: &str) -> Result<String> {
    if target_uri.starts_with("nightboat://article/") {
        return Ok(target_uri.to_string());
    }
    if let Some(synth) = super::series_service::resolve_to_synthetic_uri(pool, target_uri).await? {
        return Ok(synth);
    }
    Ok(target_uri.to_string())
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct VoteSummary {
    pub target_uri: String,
    #[ts(type = "number")]
    pub score: i64,
    #[ts(type = "number")]
    pub upvotes: i64,
    #[ts(type = "number")]
    pub downvotes: i64,
}

/// Upserts or removes an article vote inside a transaction,
/// then returns the aggregate summary atomically.
pub async fn cast_vote(
    pool: &PgPool,
    at_uri: &str,
    target_uri: &str,
    did: &str,
    value: i32,
) -> Result<VoteSummary> {
    let value = value.clamp(-1, 1);
    let target_uri = normalize_vote_target(pool, target_uri).await?;

    let mut tx = pool.begin().await?;

    if value == 0 {
        sqlx::query("DELETE FROM votes WHERE target_uri = $1 AND did = $2")
            .bind(&target_uri)
            .bind(did)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query(
            "INSERT INTO votes (at_uri, target_uri, did, value) VALUES ($1, $2, $3, $4)
             ON CONFLICT(target_uri, did) DO UPDATE SET value = EXCLUDED.value, at_uri = EXCLUDED.at_uri",
        )
        .bind(at_uri)
        .bind(&target_uri)
        .bind(did)
        .bind(value)
        .execute(&mut *tx)
        .await?;
    }

    let summary = vote_summary_in_tx(&mut tx, &target_uri).await?;

    tx.commit().await?;

    Ok(summary)
}

pub async fn get_vote_summary(pool: &PgPool, target_uri: &str) -> Result<VoteSummary> {
    let target_uri = normalize_vote_target(pool, target_uri).await?;
    let target_uri = target_uri.as_str();
    #[derive(sqlx::FromRow)]
    struct Row {
        score: i64,
        upvotes: i64,
        downvotes: i64,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT \
            COALESCE(SUM(value), 0) AS score, \
            COUNT(*) FILTER (WHERE value > 0) AS upvotes, \
            COUNT(*) FILTER (WHERE value < 0) AS downvotes \
         FROM votes WHERE target_uri = $1",
    )
    .bind(target_uri)
    .fetch_one(pool)
    .await?;

    Ok(VoteSummary {
        target_uri: target_uri.to_string(),
        score: row.score,
        upvotes: row.upvotes,
        downvotes: row.downvotes,
    })
}

async fn vote_summary_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    target_uri: &str,
) -> Result<VoteSummary> {
    #[derive(sqlx::FromRow)]
    struct Row {
        score: i64,
        upvotes: i64,
        downvotes: i64,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT \
            COALESCE(SUM(value), 0) AS score, \
            COUNT(*) FILTER (WHERE value > 0) AS upvotes, \
            COUNT(*) FILTER (WHERE value < 0) AS downvotes \
         FROM votes WHERE target_uri = $1",
    )
    .bind(target_uri)
    .fetch_one(&mut **tx)
    .await?;

    Ok(VoteSummary {
        target_uri: target_uri.to_string(),
        score: row.score,
        upvotes: row.upvotes,
        downvotes: row.downvotes,
    })
}

pub async fn get_vote_summaries_batch(pool: &PgPool, target_uris: &[String]) -> Result<Vec<VoteSummary>> {
    if target_uris.is_empty() {
        return Ok(vec![]);
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        target_uri: String,
        score: i64,
        upvotes: i64,
        downvotes: i64,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT target_uri, \
            COALESCE(SUM(value), 0) AS score, \
            COUNT(*) FILTER (WHERE value > 0) AS upvotes, \
            COUNT(*) FILTER (WHERE value < 0) AS downvotes \
         FROM votes WHERE target_uri = ANY($1) \
         GROUP BY target_uri",
    )
    .bind(target_uris)
    .fetch_all(pool)
    .await?;

    // Include entries with zero votes for URIs not in the result
    let result: Vec<VoteSummary> = rows
        .into_iter()
        .map(|r| VoteSummary {
            target_uri: r.target_uri,
            score: r.score,
            upvotes: r.upvotes,
            downvotes: r.downvotes,
        })
        .collect();

    let found: std::collections::HashSet<String> = result.iter().map(|v| v.target_uri.clone()).collect();
    let mut all = result;
    for uri in target_uris {
        if !found.contains(uri.as_str()) {
            all.push(VoteSummary {
                target_uri: uri.clone(),
                score: 0,
                upvotes: 0,
                downvotes: 0,
            });
        }
    }

    Ok(all)
}

pub async fn get_my_vote(pool: &PgPool, target_uri: &str, did: &str) -> Result<i32> {
    let target_uri = normalize_vote_target(pool, target_uri).await?;
    let value: Option<i32> = sqlx::query_scalar(
        "SELECT value FROM votes WHERE target_uri = $1 AND did = $2",
    )
    .bind(&target_uri)
    .bind(did)
    .fetch_optional(pool)
    .await?;

    Ok(value.unwrap_or(0))
}
