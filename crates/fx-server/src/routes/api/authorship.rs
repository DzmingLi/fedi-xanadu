use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use fx_core::models::ArticleAuthor;
use fx_core::services::{article_service, authorship_service};

use fx_core::util::now_rfc3339;

use crate::auth::{Auth, WriteAuth, pds_create_record};
use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use super::UriQuery;

#[derive(serde::Deserialize)]
pub struct AddAuthorInput {
    article_uri: String,
    author_did: String,
    position: Option<i16>,
}

/// Add an author to an article. The article publisher or any verified co-author can do this.
pub async fn add_author(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<AddAuthorInput>,
) -> ApiResult<StatusCode> {
    // Verify caller is the article publisher or a verified co-author
    let article = article_service::get_article(&state.pool, state.instance_mode, &input.article_uri).await
        .map_err(|_| AppError(fx_core::Error::NotFound { entity: "article", id: input.article_uri.clone() }))?;
    let is_publisher = article.author_did == user.did;
    let is_verified_author = if !is_publisher {
        let authors = authorship_service::list_authors(&state.pool, &input.article_uri).await?;
        authors.iter().any(|a| a.author_did.as_deref() == Some(&user.did) && a.status == "verified")
    } else {
        true
    };
    if !is_verified_author {
        return Err(AppError(fx_core::Error::Forbidden { action: "only the publisher or verified co-authors can add authors" }));
    }

    authorship_service::add_author(
        &state.pool,
        &input.article_uri,
        &input.author_did,
        &user.did,
        input.position,
    ).await?;

    // TODO: send notification to the added author

    Ok(StatusCode::CREATED)
}

#[derive(serde::Deserialize)]
pub struct AuthorshipActionInput {
    article_uri: String,
}

/// Verify own authorship. For AT Protocol users, creates an authorship record on their PDS.
pub async fn verify_authorship(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<AuthorshipActionInput>,
) -> ApiResult<StatusCode> {
    // Create PDS authorship record (best-effort for AT Protocol users)
    let record = serde_json::json!({
        "$type": fx_atproto::lexicon::AUTHORSHIP,
        "article": input.article_uri,
        "createdAt": now_rfc3339(),
    });
    let authorship_uri = pds_create_record(
        &state, &user.token, fx_atproto::lexicon::AUTHORSHIP, record, None, "verify authorship",
    ).await;

    let updated = authorship_service::verify_authorship(
        &state.pool,
        &input.article_uri,
        &user.did,
        authorship_uri.as_deref(),
    ).await?;

    if !updated {
        return Err(AppError(fx_core::Error::NotFound {
            entity: "pending authorship",
            id: format!("{}:{}", input.article_uri, user.did),
        }));
    }

    Ok(StatusCode::OK)
}

/// Reject authorship — "this is not me".
pub async fn reject_authorship(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<AuthorshipActionInput>,
) -> ApiResult<StatusCode> {
    let updated = authorship_service::reject_authorship(
        &state.pool,
        &input.article_uri,
        &user.did,
    ).await?;

    if !updated {
        return Err(AppError(fx_core::Error::NotFound {
            entity: "authorship",
            id: format!("{}:{}", input.article_uri, user.did),
        }));
    }

    Ok(StatusCode::OK)
}

/// List authors for an article.
pub async fn list_authors(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<UriQuery>,
) -> ApiResult<Json<Vec<ArticleAuthor>>> {
    let authors = authorship_service::list_authors(&state.pool, &q.uri).await?;
    Ok(Json(authors))
}

/// List pending authorships for the current user.
pub async fn my_pending_authorships(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let pending = authorship_service::list_pending_for_user(&state.pool, &user.did).await?;
    let result: Vec<serde_json::Value> = pending
        .into_iter()
        .map(|(uri, title)| serde_json::json!({ "article_uri": uri, "title": title }))
        .collect();
    Ok(Json(result))
}
