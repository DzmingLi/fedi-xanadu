use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use fx_core::models::{Article, CreateArticle};
use fx_core::region::default_visibility;
use fx_core::services::{appeal_service, article_service, moderation_service, notification_service, platform_user_service, series_service, tag_service};
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
        default_visibility(true), "article", None, // admin is always verified
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
    pub topics: Option<Vec<String>>,
    pub parent_id: Option<String>,
    pub lang: Option<String>,
    pub translation_of: Option<String>,
}

pub async fn admin_create_series(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AdminCreateSeriesInput>,
) -> ApiResult<(StatusCode, Json<series_service::SeriesRow>)> {
    require_admin(&state, &headers)?;

    let did = platform_user_service::local_did(&input.as_handle);
    let id = format!("s-{}", tid());

    let topics = input.topics.unwrap_or_default();
    let lang = input.lang.as_deref().unwrap_or("zh");
    let translation_group = if let Some(ref source_id) = input.translation_of {
        Some(series_service::resolve_series_translation_group(&state.pool, source_id).await?)
    } else {
        None
    };

    let row = series_service::create_series(
        &state.pool,
        &id,
        &input.title,
        input.description.as_deref(),
        &topics,
        input.parent_id.as_deref(),
        &did,
        lang,
        translation_group,
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

// --- Admin article update (bypass ownership check) ---

#[derive(serde::Deserialize)]
pub struct AdminUpdateArticleInput {
    pub uri: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
}

pub async fn admin_update_article(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AdminUpdateArticleInput>,
) -> ApiResult<Json<Article>> {
    require_admin(&state, &headers)?;

    if let Some(ref title) = input.title {
        article_service::update_article_title(&state.pool, &input.uri, title).await?;
    }
    if let Some(ref desc) = input.description {
        article_service::update_article_description(&state.pool, &input.uri, desc).await?;
    }

    if let Some(ref content) = input.content {
        let format = article_service::get_content_format(&state.pool, &input.uri).await?;

        let node_id = uri_to_node_id(&input.uri);
        let repo_path = state.pijul.repo_path(&node_id);
        let src_ext = match format.as_str() {
            "markdown" => "md",
            "html" => "html",
            _ => "typ",
        };
        tokio::fs::write(repo_path.join(format!("content.{src_ext}")), content).await?;

        if format != "html" {
            let rendered = super::articles::render_content(&format, content, &repo_path)?;
            let _ = tokio::fs::write(repo_path.join("content.html"), &rendered).await;
        }

        let hash = content_hash(content);
        article_service::update_article_content_hash(&state.pool, &input.uri, &hash).await?;

        if let Err(e) = state.pijul.record(&node_id, "Admin update") {
            tracing::warn!("pijul record failed for {node_id}: {e}");
        }
    }

    let article = article_service::get_article_any_visibility(&state.pool, &input.uri).await?;
    Ok(Json(article))
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

// --- Moderation ---

#[derive(serde::Deserialize)]
pub struct BanUserInput {
    pub did: String,
    pub reason: Option<String>,
}

pub async fn admin_ban_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<BanUserInput>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers)?;
    moderation_service::ban_user(&state.pool, &input.did, input.reason.as_deref()).await?;

    // Send in-app notification to the banned user with the reason
    let notif_id = super::tid();
    if let Err(e) = notification_service::create_notification(
        &state.pool,
        &notif_id,
        &input.did,
        "system",
        "banned",
        None,
        input.reason.as_deref(),
    ).await {
        tracing::warn!("failed to send ban notification: {e}");
    }

    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct UnbanUserInput {
    pub did: String,
}

pub async fn admin_unban_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<UnbanUserInput>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers)?;
    moderation_service::unban_user(&state.pool, &input.did).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_list_banned_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<moderation_service::BannedUser>>> {
    require_admin(&state, &headers)?;
    let users = moderation_service::list_banned_users(&state.pool).await?;
    Ok(Json(users))
}

#[derive(serde::Deserialize)]
pub struct AdminDeleteArticleInput {
    pub uri: String,
    pub reason: Option<String>,
}

pub async fn admin_delete_article(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AdminDeleteArticleInput>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers)?;

    // Fetch article info before removing so we can notify the author
    let article = article_service::get_article_any_visibility(&state.pool, &input.uri).await?;

    // Remove: article is hidden but preserved for 30-day appeal window
    article_service::set_visibility(&state.pool, &input.uri, "removed", input.reason.as_deref()).await?;

    // Notify author with title + reason
    let reason_text = match &input.reason {
        Some(r) => format!("「{}」已被删除: {}。你可以在30天内提交申诉。", article.title, r),
        None => format!("「{}」已被删除。你可以在30天内提交申诉。", article.title),
    };
    let notif_id = super::tid();
    if let Err(e) = notification_service::create_notification(
        &state.pool,
        &notif_id,
        &article.did,
        "system",
        "article_deleted",
        Some(&input.uri),
        Some(&reason_text),
    ).await {
        tracing::warn!("failed to send article deletion notification: {e}");
    }

    Ok(StatusCode::OK)
}

// --- Visibility management ---

#[derive(serde::Deserialize)]
pub struct SetVisibilityInput {
    pub uri: String,
    /// One of: public, cn_hidden, unlisted, pending_review, removed
    pub visibility: String,
    pub reason: Option<String>,
}

const VALID_VISIBILITIES: &[&str] = &["public", "cn_hidden", "removed"];

pub async fn admin_set_visibility(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<SetVisibilityInput>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers)?;

    if !VALID_VISIBILITIES.contains(&input.visibility.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest(
            format!("invalid visibility: {}. Must be one of: {}", input.visibility, VALID_VISIBILITIES.join(", ")),
        )));
    }

    article_service::set_visibility(&state.pool, &input.uri, &input.visibility, input.reason.as_deref()).await?;

    // Notify author if visibility was restricted
    if input.visibility == "removed" || input.visibility == "cn_hidden" {
        let article = article_service::get_article_any_visibility(&state.pool, &input.uri).await?;
        let msg = match input.visibility.as_str() {
            "removed" => match &input.reason {
                Some(r) => format!("「{}」已被删除: {}。你可以在30天内提交申诉。", article.title, r),
                None => format!("「{}」已被删除。你可以在30天内提交申诉。", article.title),
            },
            "cn_hidden" => match &input.reason {
                Some(r) => format!("「{}」已被设为仅国际站可见: {}", article.title, r),
                None => format!("「{}」已被设为仅国际站可见", article.title),
            },
            _ => unreachable!(),
        };
        let notif_id = super::tid();
        let _ = notification_service::create_notification(
            &state.pool, &notif_id, &article.did, "system",
            "visibility_changed", Some(&input.uri), Some(&msg),
        ).await;
    }

    Ok(StatusCode::OK)
}

// --- Question merge ---

#[derive(serde::Deserialize)]
pub struct MergeQuestionsInput {
    pub from_uri: String,
    pub into_uri: String,
}

pub async fn admin_merge_questions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<MergeQuestionsInput>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&state, &headers)?;

    let moved = article_service::merge_questions(&state.pool, &input.from_uri, &input.into_uri).await?;
    Ok(Json(serde_json::json!({
        "merged": true,
        "answers_moved": moved,
    })))
}

// --- Appeals management ---

pub async fn admin_list_appeals(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<appeal_service::Appeal>>> {
    require_admin(&state, &headers)?;
    let appeals = appeal_service::list_pending_appeals(&state.pool).await?;
    Ok(Json(appeals))
}

#[derive(serde::Deserialize)]
pub struct ResolveAppealInput {
    pub id: String,
    /// "approved" or "rejected"
    pub status: String,
    pub response: Option<String>,
}

pub async fn admin_resolve_appeal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ResolveAppealInput>,
) -> ApiResult<Json<appeal_service::Appeal>> {
    require_admin(&state, &headers)?;

    if input.status != "approved" && input.status != "rejected" {
        return Err(AppError(fx_core::Error::BadRequest(
            "status must be 'approved' or 'rejected'".to_string(),
        )));
    }

    let appeal = appeal_service::resolve_appeal(
        &state.pool,
        &input.id,
        &input.status,
        input.response.as_deref(),
    ).await?;

    // If approved, take action based on appeal kind
    if input.status == "approved" {
        match appeal.kind.as_str() {
            "ban" => {
                let _ = moderation_service::unban_user(&state.pool, &appeal.did).await;
            }
            "article_deleted" => {
                if let Some(ref uri) = appeal.target_uri {
                    let _ = article_service::set_visibility(&state.pool, uri, "public", None).await;
                }
            }
            _ => {}
        }
    }

    // Notify the user about the resolution
    let notif_id = super::tid();
    let notif_text = match (&input.status.as_str(), &input.response) {
        (&"approved", Some(r)) => format!("你的申诉已通过: {r}"),
        (&"approved", None) => "你的申诉已通过".to_string(),
        (_, Some(r)) => format!("你的申诉已被拒绝: {r}"),
        (_, None) => "你的申诉已被拒绝".to_string(),
    };
    let _ = notification_service::create_notification(
        &state.pool,
        &notif_id,
        &appeal.did,
        "system",
        "appeal_resolved",
        appeal.target_uri.as_deref(),
        Some(&notif_text),
    ).await;

    Ok(Json(appeal))
}
