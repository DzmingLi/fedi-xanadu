use axum::{
    Json,
    extract::{Query, State},
};
use fx_core::services::{vote_service, reputation_service};

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{WriteAuth, MaybeAuth, pds_create_record, pds_delete_record};
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
    let new_vote_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::VOTE, tid());
    let value = input.value.clamp(-1, 1);

    // Fetch the previous vote's at_uri (if any) before mutating so we can
    // delete the matching PDS record afterward.
    let prev_uri: Option<String> = sqlx::query_scalar(
        "SELECT at_uri FROM votes WHERE target_uri = $1 AND did = $2",
    )
    .bind(&input.target_uri)
    .bind(&user.did)
    .fetch_optional(&state.pool)
    .await?;

    let summary = vote_service::cast_vote(
        &state.pool,
        &new_vote_uri,
        &input.target_uri,
        &user.did,
        value,
    )
    .await?;

    // Update reputations: content author + voter (best-effort, non-blocking)
    let pool = state.pool.clone();
    let target = input.target_uri.clone();
    let voter = user.did.clone();
    tokio::spawn(async move {
        let _ = reputation_service::update_for_content_vote(&pool, &target, &voter).await;
    });

    // Retire the old PDS record (if switching value, also clears the stale entry)
    if let Some(pu) = prev_uri {
        if let Some(rk) = pu.rsplit('/').next() {
            pds_delete_record(&state, &user.token, fx_atproto::lexicon::VOTE, rk.to_string(), "delete old vote").await;
        }
    }

    // Publish the new vote record (only for non-zero votes)
    if value != 0 {
        let (subject, section_ref) =
            crate::routes::api::articles::resolve_subject_ref(&state.pool, &input.target_uri).await;
        let mut record = serde_json::json!({
            "$type": fx_atproto::lexicon::VOTE,
            "subject": subject,
            "value": value,
            "createdAt": now_rfc3339(),
        });
        if let Some(sr) = section_ref { record["sectionRef"] = serde_json::Value::String(sr); }
        let rkey = new_vote_uri.rsplit('/').next().map(str::to_string);
        pds_create_record(&state, &user.token, fx_atproto::lexicon::VOTE, record, rkey, "create vote").await;
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
