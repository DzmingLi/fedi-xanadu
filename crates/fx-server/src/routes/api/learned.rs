use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::learned_service;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{Auth, WriteAuth, MaybeAuth, pds_create_record, pds_delete_record};
use fx_core::util::now_rfc3339;
use super::UriQuery;

#[derive(serde::Deserialize)]
pub struct MarkLearnedInput {
    article_uri: String,
}

pub async fn mark_learned(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<MarkLearnedInput>,
) -> ApiResult<StatusCode> {
    learned_service::mark_learned(&state.pool, &user.did, &input.article_uri).await?;

    // rkey = article's TID so unmark can target it directly without an index lookup.
    let rkey = input.article_uri.rsplit('/').next().map(str::to_string);
    let (subject, section_ref) =
        fx_core::services::article_service::resolve_subject_ref(&state.pool, &input.article_uri, fx_atproto::lexicon::SERIES).await;
    let mut record = serde_json::json!({
        "$type": fx_atproto::lexicon::LEARNED,
        "subject": subject,
        "learnedAt": now_rfc3339(),
    });
    if let Some(sr) = section_ref { record["sectionRef"] = serde_json::Value::String(sr); }
    pds_create_record(&state, &user.token, fx_atproto::lexicon::LEARNED, record, rkey, "mark learned").await;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn unmark_learned(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<MarkLearnedInput>,
) -> ApiResult<StatusCode> {
    learned_service::unmark_learned(&state.pool, &user.did, &input.article_uri).await?;

    if let Some(rkey) = input.article_uri.rsplit('/').next() {
        pds_delete_record(&state, &user.token, fx_atproto::lexicon::LEARNED, rkey.to_string(), "unmark learned").await;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn is_learned(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let learned = if let Some(u) = user {
        learned_service::is_learned(&state.pool, &u.did, &uri).await?
    } else {
        false
    };
    Ok(Json(serde_json::json!({ "learned": learned })))
}

pub async fn list_learned(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<learned_service::LearnedMark>>> {
    let marks = learned_service::list_learned(&state.pool, &user.did).await?;
    Ok(Json(marks))
}
