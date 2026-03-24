use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};

use crate::error::{ApiResult, require_did};
use crate::state::AppState;
use super::{AuthDid, UriQuery, session_from_headers, tid, chrono_now};

#[derive(serde::Deserialize)]
pub struct CastVoteInput {
    target_uri: String,
    value: i32,
}

#[derive(serde::Serialize)]
pub struct VoteSummary {
    target_uri: String,
    score: i64,
    upvotes: i64,
    downvotes: i64,
}

#[derive(serde::Serialize)]
pub(crate) struct MyVoteOutput {
    value: i32,
}

pub async fn cast_vote(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    headers: HeaderMap,
    Json(input): Json<CastVoteInput>,
) -> ApiResult<Json<VoteSummary>> {
    require_did(&did)?;

    if input.value == 0 {
        sqlx::query("DELETE FROM votes WHERE target_uri = ? AND did = ?")
            .bind(&input.target_uri)
            .bind(&did)
            .execute(&state.pool)
            .await?;
    } else {
        let vote_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::VOTE, tid());
        let value = if input.value > 0 { 1 } else { -1 };

        sqlx::query(
            "INSERT INTO votes (at_uri, target_uri, did, value) VALUES (?, ?, ?, ?)
             ON CONFLICT(target_uri, did) DO UPDATE SET value = ?, at_uri = ?",
        )
        .bind(&vote_uri)
        .bind(&input.target_uri)
        .bind(&did)
        .bind(value)
        .bind(value)
        .bind(&vote_uri)
        .execute(&state.pool)
        .await?;

        if let Some((_did, pds_url, access_jwt)) = session_from_headers(&state.pool, &headers).await {
            let record = serde_json::json!({
                "$type": fx_atproto::lexicon::VOTE,
                "subject": input.target_uri,
                "value": value,
                "createdAt": chrono_now(),
            });
            let _ = state.at_client.create_record(
                &pds_url,
                &access_jwt,
                &fx_atproto::client::CreateRecordInput {
                    repo: _did,
                    collection: fx_atproto::lexicon::VOTE.to_string(),
                    record,
                    rkey: None,
                },
            ).await;
        }
    }

    let summary = get_vote_summary(&state.pool, &input.target_uri).await?;
    Ok(Json(summary))
}

pub async fn get_article_votes(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<VoteSummary>> {
    let summary = get_vote_summary(&state.pool, &uri).await?;
    Ok(Json(summary))
}

pub async fn get_my_vote(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<MyVoteOutput>> {
    let value: Option<i32> = sqlx::query_scalar(
        "SELECT value FROM votes WHERE target_uri = ? AND did = ?",
    )
    .bind(&uri)
    .bind(&did)
    .fetch_optional(&state.pool)
    .await?;

    Ok(Json(MyVoteOutput { value: value.unwrap_or(0) }))
}

async fn get_vote_summary(pool: &sqlx::SqlitePool, target_uri: &str) -> anyhow::Result<VoteSummary> {
    let score: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(value), 0) FROM votes WHERE target_uri = ?",
    )
    .bind(target_uri)
    .fetch_one(pool)
    .await?;

    let upvotes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM votes WHERE target_uri = ? AND value > 0",
    )
    .bind(target_uri)
    .fetch_one(pool)
    .await?;

    let downvotes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM votes WHERE target_uri = ? AND value < 0",
    )
    .bind(target_uri)
    .fetch_one(pool)
    .await?;

    Ok(VoteSummary {
        target_uri: target_uri.to_string(),
        score,
        upvotes,
        downvotes,
    })
}
