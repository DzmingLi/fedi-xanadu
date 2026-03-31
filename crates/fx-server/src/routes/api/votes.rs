use axum::{
    Json,
    extract::{Query, State},
};
use fx_core::services::vote_service;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{WriteAuth, MaybeAuth, pds_create_record};
use fx_core::util::{tid, now_rfc3339};
use super::UriQuery;

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
    WriteAuth(user): WriteAuth,
    Json(input): Json<CastVoteInput>,
) -> ApiResult<Json<VoteSummary>> {
    let vote_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::VOTE, tid());
    let value = input.value.clamp(-1, 1);

    let summary = vote_service::cast_vote(
        &state.pool,
        &vote_uri,
        &input.target_uri,
        &user.did,
        value,
    )
    .await?;

    // AT Protocol side-effect (only for non-zero votes)
    if value != 0 {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::VOTE,
            "subject": input.target_uri,
            "value": value,
            "createdAt": now_rfc3339(),
        });
        pds_create_record(&state, &user.token, fx_atproto::lexicon::VOTE, record, None, "create vote").await;
    }

    Ok(Json(VoteSummary {
        target_uri: summary.target_uri,
        score: summary.score,
        upvotes: summary.upvotes,
        downvotes: summary.downvotes,
    }))
}

pub async fn get_article_votes(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<VoteSummary>> {
    let summary = vote_service::get_vote_summary(&state.pool, &uri).await?;
    Ok(Json(VoteSummary {
        target_uri: summary.target_uri,
        score: summary.score,
        upvotes: summary.upvotes,
        downvotes: summary.downvotes,
    }))
}

pub async fn get_votes_batch(
    State(state): State<AppState>,
    Json(uris): Json<Vec<String>>,
) -> ApiResult<Json<Vec<VoteSummary>>> {
    let summaries = vote_service::get_vote_summaries_batch(&state.pool, &uris).await?;
    Ok(Json(
        summaries
            .into_iter()
            .map(|s| VoteSummary {
                target_uri: s.target_uri,
                score: s.score,
                upvotes: s.upvotes,
                downvotes: s.downvotes,
            })
            .collect(),
    ))
}

pub async fn get_my_vote(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<MyVoteOutput>> {
    let did = user.map(|u| u.did).unwrap_or_default();
    let value = vote_service::get_my_vote(&state.pool, &uri, &did).await?;
    Ok(Json(MyVoteOutput { value }))
}
