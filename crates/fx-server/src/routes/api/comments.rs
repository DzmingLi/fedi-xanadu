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
use crate::auth::{WriteAuth, MaybeAuth};
use fx_core::util::tid;
use super::UriQuery;

#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct ListCommentsQuery {
    pub uri: String,
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/comments",
    params(ListCommentsQuery),
    responses((status = 200, description = "List of comments", body = Vec<Comment>))
)]
pub async fn list_comments(
    State(state): State<AppState>,
    Query(q): Query<ListCommentsQuery>,
) -> ApiResult<Json<Vec<Comment>>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);
    let comments = comment_service::list_comments(&state.pool, &q.uri, limit).await?;
    Ok(Json(comments))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CreateComment {
    pub content_uri: String,
    pub body: String,
    pub parent_id: Option<String>,
    pub quote_text: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/comments",
    request_body = CreateComment,
    responses(
        (status = 201, description = "Comment created", body = Comment),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Validation error"),
    ),
    security(("bearer" = []))
)]
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

    let id = tid();
    let comment = comment_service::create_comment(
        &state.pool,
        &id,
        &input.content_uri,
        &user.did,
        &input.body,
        input.parent_id.as_deref(),
        input.quote_text.as_deref(),
    )
    .await?;

    // Notify content author
    if let Ok(Some(content_did)) = sqlx::query_scalar::<_, String>(
        "SELECT did FROM articles WHERE at_uri = $1"
    ).bind(&input.content_uri).fetch_optional(&state.pool).await {
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

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateComment {
    pub body: String,
}

#[utoipa::path(
    put,
    path = "/api/v1/comments/{id}",
    params(("id" = String, Path, description = "Comment ID")),
    request_body = UpdateComment,
    responses(
        (status = 200, description = "Comment updated", body = Comment),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = []))
)]
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
    Ok(Json(comment))
}

#[utoipa::path(
    delete,
    path = "/api/v1/comments/{id}",
    params(("id" = String, Path, description = "Comment ID")),
    responses(
        (status = 204, description = "Comment deleted"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = []))
)]
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

    comment_service::delete_comment(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Comment votes ---

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CommentVoteInput {
    pub value: i32,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct CommentVoteResult {
    pub comment_id: String,
    pub score: i64,
    pub my_vote: i32,
}

#[utoipa::path(
    post,
    path = "/api/v1/comments/{id}/vote",
    params(("id" = String, Path, description = "Comment ID")),
    request_body = CommentVoteInput,
    responses((status = 200, description = "Comment vote result", body = CommentVoteResult)),
    security(("bearer" = []))
)]
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

#[utoipa::path(
    get,
    path = "/api/v1/comments/my-votes",
    params(("uri" = String, Query, description = "Content URI")),
    responses((status = 200, body = Vec<comment_service::MyCommentVote>))
)]
pub async fn get_my_comment_votes(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<comment_service::MyCommentVote>>> {
    let did = user.map(|u| u.did).unwrap_or_default();
    let votes = comment_service::get_my_comment_votes(&state.pool, &did, &uri).await?;
    Ok(Json(votes))
}
