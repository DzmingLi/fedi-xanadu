use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fx_core::models::Comment;
use fx_core::services::{comment_service, notification_service};
use fx_core::validation::validate_comment_body;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::{WriteAuth, MaybeAuth, pds_create_record, pds_put_record, pds_delete_record};
use fx_core::util::{tid, now_rfc3339};
use super::UriQuery;

#[derive(serde::Deserialize)]
pub struct ListCommentsQuery {
    pub uri: String,
    pub section_ref: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_comments(
    State(state): State<AppState>,
    Query(q): Query<ListCommentsQuery>,
) -> ApiResult<Json<Vec<Comment>>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);
    let comments = comment_service::list_comments(&state.pool, &q.uri, q.section_ref.as_deref(), limit).await?;
    Ok(Json(comments))
}

#[derive(serde::Deserialize)]
pub struct CreateComment {
    pub content_uri: String,
    pub title: Option<String>,
    pub body: String,
    pub parent_id: Option<String>,
    pub quote_text: Option<String>,
    pub section_ref: Option<String>,
}

pub async fn create_comment(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateComment>,
) -> ApiResult<(StatusCode, Json<Comment>)> {
    validate_comment_body(&input.body)
        .map_err(|e| AppError(fx_core::Error::Validation(vec![e])))?;

    if let Some(ref pid) = input.parent_id {
        comment_service::verify_parent_comment(&state.pool, pid, &input.content_uri).await?;
    }

    let title = input.title.as_deref().map(str::trim).filter(|s| !s.is_empty());

    let id = tid();
    let comment = comment_service::create_comment(
        &state.pool,
        &id,
        &input.content_uri,
        &user.did,
        title,
        &input.body,
        input.parent_id.as_deref(),
        input.quote_text.as_deref(),
        input.section_ref.as_deref(),
    )
    .await?;

    // Publish comment to the commenter's PDS — rkey == DB id so future
    // update/delete can target it directly.
    let parent_at_uri = if let Some(ref pid) = input.parent_id {
        sqlx::query_scalar::<_, (String, String)>(
            "SELECT did, id FROM comments WHERE id = $1",
        )
        .bind(pid)
        .fetch_optional(&state.pool)
        .await?
        .map(|(d, i)| format!("at://{}/{}/{}", d, fx_atproto::lexicon::COMMENT, i))
    } else {
        None
    };
    let (subject, chapter_ref) =
        fx_core::services::article_service::resolve_subject_ref(&state.pool, &input.content_uri, fx_atproto::lexicon::SERIES).await;
    let mut record = serde_json::json!({
        "$type": fx_atproto::lexicon::COMMENT,
        "subject": subject,
        "body": input.body,
        "createdAt": now_rfc3339(),
    });
    if let Some(t) = title { record["title"] = serde_json::Value::String(t.to_string()); }
    if let Some(p) = parent_at_uri { record["parent"] = serde_json::Value::String(p); }
    if let Some(q) = input.quote_text.as_deref() { record["quoteText"] = serde_json::Value::String(q.to_string()); }
    // Explicit section_ref (paragraph anchor) from the request wins over the
    // chapter-level ref; otherwise carry the chapter tid so the comment stays
    // attached to its chapter when served under the series record.
    let section = input.section_ref.as_deref().map(str::to_string).or(chapter_ref);
    if let Some(s) = section { record["sectionRef"] = serde_json::Value::String(s); }
    pds_create_record(&state, &user.token, fx_atproto::lexicon::COMMENT, record, Some(id.clone()), "create comment").await;

    // Notify content author
    if let Ok(content_did) = fx_core::services::article_service::get_article_owner(&state.pool, &input.content_uri).await {
        if let Err(e) = notification_service::create_notification(
            &state.pool, &tid(), &content_did, &user.did,
            "article_comment", Some(&input.content_uri), Some(&id),
        ).await {
            tracing::warn!("notification failed: {e}");
        }
    }

    // Notify parent comment author (reply)
    if let Some(ref pid) = input.parent_id {
        if let Ok(Some(parent_did)) = sqlx::query_scalar::<_, String>(
            "SELECT did FROM comments WHERE id = $1"
        ).bind(pid).fetch_optional(&state.pool).await {
            if let Err(e) = notification_service::create_notification(
                &state.pool, &tid(), &parent_did, &user.did,
                "comment_reply", Some(&input.content_uri), Some(&id),
            ).await {
                tracing::warn!("notification failed: {e}");
            }
        }
    }

    Ok((StatusCode::CREATED, Json(comment)))
}

#[derive(serde::Deserialize)]
pub struct UpdateComment {
    pub body: String,
}

pub async fn update_comment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UpdateComment>,
) -> ApiResult<Json<Comment>> {
    validate_comment_body(&input.body)
        .map_err(|e| AppError(fx_core::Error::Validation(vec![e])))?;

    let (comment_did, _article_did) =
        comment_service::get_comment_owner(&state.pool, &id).await?;
    if comment_did != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "update comment owned by another user" }));
    }

    let comment = comment_service::update_comment(&state.pool, &id, &input.body).await?;

    // Overwrite the record on PDS with refreshed body + updatedAt.
    let (subject, chapter_ref) =
        fx_core::services::article_service::resolve_subject_ref(&state.pool, &comment.content_uri, fx_atproto::lexicon::SERIES).await;
    let mut record = serde_json::json!({
        "$type": fx_atproto::lexicon::COMMENT,
        "subject": subject,
        "body": comment.body,
        "createdAt": comment.created_at,
        "updatedAt": now_rfc3339(),
    });
    if let Some(ref t) = comment.title { record["title"] = serde_json::Value::String(t.clone()); }
    if let Some(ref p) = comment.parent_id {
        if let Ok(Some(pdid)) = sqlx::query_scalar::<_, String>(
            "SELECT did FROM comments WHERE id = $1",
        ).bind(p).fetch_optional(&state.pool).await {
            record["parent"] = serde_json::Value::String(format!("at://{}/{}/{}", pdid, fx_atproto::lexicon::COMMENT, p));
        }
    }
    if let Some(ref q) = comment.quote_text { record["quoteText"] = serde_json::Value::String(q.clone()); }
    let section = comment.section_ref.clone().or(chapter_ref);
    if let Some(s) = section { record["sectionRef"] = serde_json::Value::String(s); }
    pds_put_record(&state, &user.token, fx_atproto::lexicon::COMMENT, id.clone(), record, "update comment").await;

    Ok(Json(comment))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
) -> ApiResult<StatusCode> {
    let (comment_did, article_did) =
        comment_service::get_comment_owner(&state.pool, &id).await?;

    // Comment author or article author may delete
    if comment_did != user.did && article_did != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "delete comment" }));
    }

    // Only the commenter has write access to the commenter's PDS. Article
    // authors can still delete the local DB row, but the PDS record remains
    // under the commenter's repo (source of truth for that user).
    if comment_did == user.did {
        pds_delete_record(&state, &user.token, fx_atproto::lexicon::COMMENT, id.clone(), "delete comment").await;
    }

    comment_service::delete_comment(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Comment votes ---

#[derive(serde::Deserialize)]
pub struct CommentVoteInput {
    pub value: i32,
}

#[derive(serde::Serialize)]
pub struct CommentVoteResult {
    pub comment_id: String,
    pub score: i64,
    pub my_vote: i32,
}

pub async fn vote_comment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CommentVoteInput>,
) -> ApiResult<Json<CommentVoteResult>> {
    let value = input.value.clamp(-1, 1);
    let score = comment_service::vote_comment(&state.pool, &id, &user.did, value).await?;

    Ok(Json(CommentVoteResult {
        comment_id: id,
        score,
        my_vote: value,
    }))
}

pub async fn get_my_comment_votes(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<comment_service::MyCommentVote>>> {
    let did = user.map(|u| u.did).unwrap_or_default();
    let votes = comment_service::get_my_comment_votes(&state.pool, &did, &uri).await?;
    Ok(Json(votes))
}
