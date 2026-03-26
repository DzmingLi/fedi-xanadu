use axum::{
    Json,
    extract::{Query, State},
};
use fx_core::services::vote_service;

use crate::error::ApiResult;
use crate::state::AppState;
use super::{Auth, MaybeAuth, UriQuery, pds_session, tid, now_rfc3339, log_pds_error};

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
    Auth(user): Auth,
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
        if let Some(pds) = pds_session(&state.pool, &user.token).await {
            let record = serde_json::json!({
                "$type": fx_atproto::lexicon::VOTE,
                "subject": input.target_uri,
                "value": value,
                "createdAt": now_rfc3339(),
            });
            if let Err(e) = state.at_client.create_record(
                &pds.pds_url,
                &pds.access_jwt,
                &fx_atproto::client::CreateRecordInput {
                    repo: pds.did,
                    collection: fx_atproto::lexicon::VOTE.to_string(),
                    record,
                    rkey: None,
                },
            ).await {
                log_pds_error("create vote", e);
            }
        }
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

pub async fn get_my_vote(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<MyVoteOutput>> {
    let did = user.map(|u| u.did).unwrap_or_default();
    let value = vote_service::get_my_vote(&state.pool, &uri, &did).await?;
    Ok(Json(MyVoteOutput { value }))
}
