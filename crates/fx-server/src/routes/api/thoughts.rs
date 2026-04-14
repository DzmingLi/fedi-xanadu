use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::content::ContentKind;
use fx_core::models::*;
use fx_core::region::default_visibility;
use fx_core::services::{article_service, version_service};
use fx_core::validation::validate_create_thought;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{WriteAuth, pds_create_record};
use fx_core::util::{content_hash, tid, now_rfc3339};

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
    let hash = content_hash(&input.content);

    // Store article metadata in DB (no pijul repo for thoughts)
    let article = article_service::create_article(
        &state.pool, &user.did, &at_uri, &input, &hash, None,
        default_visibility(user.phone_verified), ContentKind::Thought, None,
    ).await?;

    // Store source text directly in article_versions (single version, no pijul)
    let _ = version_service::record_version(
        &state.pool, &at_uri, &hash, &user.did, "Published", &input.content,
    ).await;

    // Sync to PDS — thoughts are lightweight, content fits in the record
    if !input.restricted.unwrap_or(false) {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::ARTICLE,
            "title": input.title,
            "description": input.description.as_deref().unwrap_or(""),
            "contentFormat": input.content_format,
            "contentSource": input.content,
            "tags": input.tags,
            "kind": "thought",
            "createdAt": now_rfc3339(),
        });
        pds_create_record(&state, &user.token, fx_atproto::lexicon::ARTICLE, record, None, "create thought").await;
    }

    let _ = article_service::auto_bookmark(&state.pool, &user.did, &at_uri).await;

    Ok((StatusCode::CREATED, Json(article)))
}
