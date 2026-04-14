use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::content::ContentKind;
use fx_core::models::*;
use fx_core::region::default_visibility;
use fx_core::services::article_service;
use fx_core::validation::validate_create_thought;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::WriteAuth;
use fx_core::util::{content_hash, tid};

#[derive(serde::Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_thoughts(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let offset = q.offset.unwrap_or(0).max(0);
    let thoughts = article_service::list_thoughts(&state.pool, state.instance_mode, limit, offset).await?;
    Ok(Json(thoughts))
}

pub async fn create_thought(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateArticle>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    validate_create_thought(&input)?;

    let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::ARTICLE, tid());

    super::articles::publish_article_content(
        &state, &at_uri, &user.did, &user.token, &input.content, input.content_format,
        None, "Initial publish",
    ).await?;

    let hash = content_hash(&input.content);
    let article = article_service::create_article(
        &state.pool, &user.did, &at_uri, &input, &hash, None,
        default_visibility(user.phone_verified), ContentKind::Thought, None,
    ).await?;

    let _ = article_service::auto_bookmark(&state.pool, &user.did, &at_uri).await;

    Ok((StatusCode::CREATED, Json(article)))
}
