use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use fx_core::models::{Article, CreateArticle};
use fx_core::services::{article_service, platform_user_service, series_service, tag_service};
use fx_core::validation::validate_create_article;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use super::{content_hash, tid, uri_to_node_id};

fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    let secret = state.admin_secret.as_deref()
        .ok_or(AppError(fx_core::Error::Forbidden { action: "admin not configured" }))?;
    let provided = headers.get("x-admin-secret")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError(fx_core::Error::Unauthorized))?;
    if provided != secret {
        return Err(AppError(fx_core::Error::Unauthorized));
    }
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct CreatePlatformUserInput {
    handle: String,
    password: String,
    display_name: Option<String>,
}

pub async fn create_platform_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreatePlatformUserInput>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers)?;

    let did = platform_user_service::create_platform_user(
        &state.pool,
        &input.handle,
        input.display_name.as_deref(),
        &input.password,
    ).await?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "did": did,
        "handle": input.handle,
    }))))
}

pub async fn list_platform_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<platform_user_service::PlatformUserInfo>>> {
    require_admin(&state, &headers)?;
    let users = platform_user_service::list_platform_users(&state.pool).await?;
    Ok(Json(users))
}

// --- Admin article creation (publish as any platform user) ---

#[derive(serde::Deserialize)]
pub struct AdminCreateArticleInput {
    /// Platform user handle to publish as
    pub as_handle: String,
    #[serde(flatten)]
    pub article: CreateArticle,
}

pub async fn admin_create_article(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AdminCreateArticleInput>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    require_admin(&state, &headers)?;
    validate_create_article(&input.article)?;

    // Resolve handle → DID
    let did = platform_user_service::local_did(&input.as_handle);

    let at_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::ARTICLE, tid());

    // Init pijul repo and write source file
    let node_id = uri_to_node_id(&at_uri);
    state.pijul.init_repo(&node_id)
        .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;

    let repo_path = state.pijul.repo_path(&node_id);
    let src_ext = match input.article.content_format.as_str() {
        "markdown" => "md",
        "html" => "html",
        _ => "typ",
    };
    tokio::fs::write(repo_path.join(format!("content.{src_ext}")), &input.article.content).await?;

    // Pre-render HTML cache
    if input.article.content_format != "html" {
        let rendered = super::articles::render_content(
            &input.article.content_format, &input.article.content, &repo_path,
        )?;
        let _ = tokio::fs::write(repo_path.join("content.html"), &rendered).await;
    }

    if let Err(e) = state.pijul.record(&node_id, "Initial publish") {
        tracing::warn!("pijul record failed for {node_id}: {e}");
    }

    let hash = content_hash(&input.article.content);

    let translation_group = if let Some(ref source_uri) = input.article.translation_of {
        Some(article_service::resolve_translation_group(&state.pool, source_uri).await?)
    } else {
        None
    };

    let article = article_service::create_article(
        &state.pool, &did, &at_uri, &input.article, &hash, translation_group,
    ).await?;

    // Auto-bookmark
    let _ = article_service::auto_bookmark(&state.pool, &did, &at_uri).await;

    Ok((StatusCode::CREATED, Json(article)))
}

// --- Admin series management ---

#[derive(serde::Deserialize)]
pub struct AdminCreateSeriesInput {
    pub as_handle: String,
    pub title: String,
    pub description: Option<String>,
    pub tag_id: String,
    pub parent_id: Option<String>,
}

pub async fn admin_create_series(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AdminCreateSeriesInput>,
) -> ApiResult<(StatusCode, Json<series_service::SeriesRow>)> {
    require_admin(&state, &headers)?;

    let did = platform_user_service::local_did(&input.as_handle);
    let id = format!("s-{}", tid());

    let row = series_service::create_series(
        &state.pool,
        &id,
        &input.title,
        input.description.as_deref(),
        &input.tag_id,
        input.parent_id.as_deref(),
        &did,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(row)))
}

#[derive(serde::Deserialize)]
pub struct AdminAddSeriesArticleInput {
    pub series_id: String,
    pub article_uri: String,
}

pub async fn admin_add_series_article(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AdminAddSeriesArticleInput>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers)?;

    series_service::add_series_article(&state.pool, &input.series_id, &input.article_uri).await?;
    Ok(StatusCode::OK)
}

// --- Admin tag merge ---

#[derive(serde::Deserialize)]
pub struct MergeTagInput {
    pub from: String,
    pub into: String,
}

pub async fn admin_merge_tag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<MergeTagInput>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers)?;

    tag_service::merge_tag(&state.pool, &input.from, &input.into).await?;
    Ok(StatusCode::OK)
}
