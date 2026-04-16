use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use fx_core::content::ContentKind;
use fx_core::models::{Article, CreateArticle};
use fx_core::region::default_visibility;
use fx_core::services::{appeal_service, article_service, moderation_service, notification_service, platform_user_service, report_service, series_service, tag_service};
use fx_core::validation::validate_create_article;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::AdminAuth;
use fx_core::util::{content_hash, tid};

#[derive(serde::Deserialize)]
pub struct CreatePlatformUserInput {
    handle: String,
    password: String,
    display_name: Option<String>,
}

pub async fn create_platform_user(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<CreatePlatformUserInput>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {


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
    _admin: AdminAuth,
) -> ApiResult<Json<Vec<platform_user_service::PlatformUserInfo>>> {

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
    _admin: AdminAuth,
    Json(input): Json<AdminCreateArticleInput>,
) -> ApiResult<(StatusCode, Json<Article>)> {

    validate_create_article(&input.article)?;

    let did = platform_user_service::local_did(&input.as_handle);
    let at_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::ARTICLE, tid());

    super::articles::publish_article_content(
        &state, &at_uri, &did, "", &input.article.content, input.article.content_format,
        input.article.series_id.as_deref(), "Initial publish",
    ).await?;

    let hash = content_hash(&input.article.content);
    let translation_group = if let Some(ref source_uri) = input.article.translation_of {
        Some(article_service::resolve_translation_group(&state.pool, source_uri).await?)
    } else {
        None
    };

    let article = article_service::create_article(
        &state.pool, &did, &at_uri, &input.article, &hash, translation_group,
        default_visibility(true), ContentKind::Article, None,
    ).await?;

    if let Some(ref sid) = input.article.series_id {
        series_service::add_series_article(&state.pool, sid, &at_uri).await?;
    }

    let _ = article_service::auto_bookmark(&state.pool, &did, &at_uri).await;

    Ok((StatusCode::CREATED, Json(article)))
}

// --- Admin series management ---

#[derive(serde::Deserialize)]
pub struct AdminCreateSeriesInput {
    pub as_handle: String,
    pub title: String,
    pub description: Option<String>,
    pub long_description: Option<String>,
    pub topics: Option<Vec<String>>,
    pub lang: Option<String>,
    pub translation_of: Option<String>,
    pub category: Option<String>,
}

pub async fn admin_create_series(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<AdminCreateSeriesInput>,
) -> ApiResult<(StatusCode, Json<series_service::SeriesRow>)> {


    let did = platform_user_service::local_did(&input.as_handle);
    let id = format!("s-{}", tid());

    let topics = input.topics.unwrap_or_default();
    let lang = input.lang.as_deref().unwrap_or("zh");
    let translation_group = if let Some(ref source_id) = input.translation_of {
        Some(series_service::resolve_series_translation_group(&state.pool, source_id).await?)
    } else {
        None
    };

    let category = input.category.as_deref().unwrap_or("general");

    // Init pijul repo for all series
    let node_id = format!("series_{id}");
    if let Err(e) = state.pijul.init_series_repo(&node_id) {
        tracing::warn!("failed to init series pijul repo: {e}");
    }

    let row = series_service::create_series(
        &state.pool,
        &id,
        &input.title,
        input.description.as_deref(),
        input.long_description.as_deref(),
        &topics,
        &did,
        lang,
        translation_group,
        category,
        Some(&node_id),
    )
    .await?;

    // Admin-created series are published by default
    series_service::publish_series(&state.pool, &id).await?;

    Ok((StatusCode::CREATED, Json(row)))
}

#[derive(serde::Deserialize)]
pub struct AdminAddSeriesArticleInput {
    pub series_id: String,
    pub article_uri: String,
}

pub async fn admin_add_series_article(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<AdminAddSeriesArticleInput>,
) -> ApiResult<StatusCode> {


    series_service::add_series_article(&state.pool, &input.series_id, &input.article_uri).await?;
    Ok(StatusCode::NO_CONTENT)
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
    _admin: AdminAuth,
    Json(input): Json<AdminUpdateArticleInput>,
) -> ApiResult<Json<Article>> {


    if let Some(ref title) = input.title {
        article_service::update_article_title(&state.pool, &input.uri, title).await?;
    }
    if let Some(ref desc) = input.description {
        article_service::update_article_description(&state.pool, &input.uri, desc).await?;
    }

    if let Some(ref content) = input.content {
        let format = article_service::get_content_format(&state.pool, &input.uri).await?;
        super::articles::update_article_content(&state, &input.uri, "admin", None, content, &format, "Admin update").await?;
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
    _admin: AdminAuth,
    Json(input): Json<MergeTagInput>,
) -> ApiResult<StatusCode> {


    tag_service::merge_tag(&state.pool, &input.from, &input.into).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Moderation ---

#[derive(serde::Deserialize)]
pub struct BanUserInput {
    pub did: String,
    pub reason: Option<String>,
}

pub async fn admin_ban_user(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<BanUserInput>,
) -> ApiResult<StatusCode> {

    moderation_service::ban_user(&state.pool, &input.did, input.reason.as_deref()).await?;

    // Send in-app notification to the banned user with the reason
    let notif_id = tid();
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

    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct UnbanUserInput {
    pub did: String,
}

pub async fn admin_unban_user(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<UnbanUserInput>,
) -> ApiResult<StatusCode> {

    moderation_service::unban_user(&state.pool, &input.did).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn admin_list_banned_users(
    State(state): State<AppState>,
    _admin: AdminAuth,
) -> ApiResult<Json<Vec<moderation_service::BannedUser>>> {

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
    _admin: AdminAuth,
    Json(input): Json<AdminDeleteArticleInput>,
) -> ApiResult<StatusCode> {


    // Fetch article info before removing so we can notify the author
    let article = article_service::get_article_any_visibility(&state.pool, &input.uri).await?;

    // Remove: article is hidden but preserved for 30-day appeal window
    article_service::set_visibility(&state.pool, &input.uri, "removed", input.reason.as_deref()).await?;

    // Notify author with title + reason
    let reason_text = match &input.reason {
        Some(r) => format!("「{}」已被删除: {}。你可以在30天内提交申诉。", article.title, r),
        None => format!("「{}」已被删除。你可以在30天内提交申诉。", article.title),
    };
    let notif_id = tid();
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

    Ok(StatusCode::NO_CONTENT)
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
    _admin: AdminAuth,
    Json(input): Json<SetVisibilityInput>,
) -> ApiResult<StatusCode> {


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
        let notif_id = tid();
        let _ = notification_service::create_notification(
            &state.pool, &notif_id, &article.did, "system",
            "visibility_changed", Some(&input.uri), Some(&msg),
        ).await;
    }

    Ok(StatusCode::NO_CONTENT)
}

// --- Question merge ---

#[derive(serde::Deserialize)]
pub struct MergeQuestionsInput {
    pub from_uri: String,
    pub into_uri: String,
}

pub async fn admin_merge_questions(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<MergeQuestionsInput>,
) -> ApiResult<Json<serde_json::Value>> {


    let moved = article_service::merge_questions(&state.pool, &input.from_uri, &input.into_uri).await?;
    Ok(Json(serde_json::json!({
        "merged": true,
        "answers_moved": moved,
    })))
}

// --- Appeals management ---

pub async fn admin_list_appeals(
    State(state): State<AppState>,
    _admin: AdminAuth,
) -> ApiResult<Json<Vec<appeal_service::Appeal>>> {

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
    _admin: AdminAuth,
    Json(input): Json<ResolveAppealInput>,
) -> ApiResult<Json<appeal_service::Appeal>> {


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
    let notif_id = tid();
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

// ---- Reports ----

#[derive(serde::Deserialize)]
pub struct ListReportsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

pub async fn admin_list_reports(
    State(state): State<AppState>,
    _admin: AdminAuth,
    axum::extract::Query(q): axum::extract::Query<ListReportsQuery>,
) -> ApiResult<Json<Vec<report_service::ReportWithNames>>> {

    let reports = report_service::list_reports(
        &state.pool,
        q.status.as_deref(),
        q.limit.unwrap_or(100),
    )
    .await?;
    Ok(Json(reports))
}

#[derive(serde::Deserialize)]
pub struct ResolveReportInput {
    pub id: String,
    pub status: String,
    pub admin_note: Option<String>,
}

pub async fn admin_resolve_report(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<ResolveReportInput>,
) -> ApiResult<Json<serde_json::Value>> {

    report_service::resolve_report(
        &state.pool,
        &input.id,
        &input.status,
        input.admin_note.as_deref(),
    )
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---- Credentials Verification (education + affiliation) ----

#[derive(serde::Deserialize)]
pub struct VerifyCredentialsInput {
    pub did: String,
    /// Education entries: [{degree, school, year, current}]
    pub education: Option<serde_json::Value>,
    /// Current affiliation (workplace / org)
    pub affiliation: Option<String>,
}

pub async fn admin_verify_credentials(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<VerifyCredentialsInput>,
) -> ApiResult<Json<serde_json::Value>> {


    let education = input.education.unwrap_or(serde_json::json!([]));

    // Update profiles
    sqlx::query(
        "UPDATE profiles SET education = $1, affiliation = $2, credentials_verified = true, credentials_verified_at = NOW() WHERE did = $3",
    )
    .bind(&education)
    .bind(&input.affiliation)
    .bind(&input.did)
    .execute(&state.pool)
    .await?;

    // Update platform_users
    sqlx::query(
        "UPDATE platform_users SET education = $1, affiliation = $2, credentials_verified = true, credentials_verified_at = NOW() WHERE did = $3",
    )
    .bind(&education)
    .bind(&input.affiliation)
    .bind(&input.did)
    .execute(&state.pool)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true, "did": input.did })))
}

pub async fn admin_revoke_credentials(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {

    let did = input["did"].as_str()
        .ok_or(AppError(fx_core::Error::BadRequest("did required".into())))?;

    sqlx::query("UPDATE profiles SET education = '[]', affiliation = NULL, credentials_verified = false, credentials_verified_at = NULL WHERE did = $1")
        .bind(did)
        .execute(&state.pool)
        .await?;

    sqlx::query("UPDATE platform_users SET education = '[]', affiliation = NULL, credentials_verified = false, credentials_verified_at = NULL WHERE did = $1")
        .bind(did)
        .execute(&state.pool)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---- Admin Question/Answer Publishing ----

#[derive(serde::Deserialize)]
pub struct AdminCreateQuestionInput {
    pub as_handle: String,
    #[serde(flatten)]
    pub article: CreateArticle,
}

pub async fn admin_create_question(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<AdminCreateQuestionInput>,
) -> ApiResult<(StatusCode, Json<Article>)> {

    validate_create_article(&input.article)?;

    let did = platform_user_service::local_did(&input.as_handle);
    let at_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::ARTICLE, tid());

    super::articles::publish_article_content(
        &state, &at_uri, &did, "", &input.article.content, input.article.content_format,
        None, "Initial publish",
    ).await?;

    let hash = content_hash(&input.article.content);

    let article = article_service::create_article(
        &state.pool, &did, &at_uri, &input.article, &hash, None,
        default_visibility(true), ContentKind::Question, None,
    ).await?;

    let _ = article_service::auto_bookmark(&state.pool, &did, &at_uri).await;

    Ok((StatusCode::CREATED, Json(article)))
}

#[derive(serde::Deserialize)]
pub struct AdminPostAnswerInput {
    pub as_handle: String,
    pub question_uri: String,
    #[serde(flatten)]
    pub article: CreateArticle,
}

pub async fn admin_post_answer(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<AdminPostAnswerInput>,
) -> ApiResult<(StatusCode, Json<Article>)> {

    validate_create_article(&input.article)?;

    // Verify question exists
    let question = article_service::get_article_any_visibility(&state.pool, &input.question_uri).await?;
    if question.kind != ContentKind::Question {
        return Err(AppError(fx_core::Error::BadRequest("target is not a question".into())));
    }

    let did = platform_user_service::local_did(&input.as_handle);
    let at_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::ARTICLE, tid());

    super::articles::publish_article_content(
        &state, &at_uri, &did, "", &input.article.content, input.article.content_format,
        None, "Initial publish",
    ).await?;

    let hash = content_hash(&input.article.content);

    let article = article_service::create_article(
        &state.pool, &did, &at_uri, &input.article, &hash, None,
        default_visibility(true), ContentKind::Answer, Some(&input.question_uri),
    ).await?;

    // Notify question author
    if question.did != did {
        let notif_id = tid();
        let _ = notification_service::create_notification(
            &state.pool, &notif_id, &question.did, &did,
            "new_answer", Some(&input.question_uri), Some(&at_uri),
        ).await;
    }

    Ok((StatusCode::CREATED, Json(article)))
}

// --- Revert book edit ---

#[derive(serde::Deserialize)]
pub struct RevertBookEditInput {
    pub edit_id: String,
}

pub async fn admin_revert_book_edit(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<RevertBookEditInput>,
) -> ApiResult<Json<serde_json::Value>> {


    let log: super::books::BookEditLog = sqlx::query_as(
        "SELECT l.id, l.book_id, l.editor_did, p.handle AS editor_handle, \
                l.old_data, l.new_data, l.summary, l.created_at \
         FROM book_edit_log l \
         LEFT JOIN profiles p ON l.editor_did = p.did \
         WHERE l.id = $1",
    )
    .bind(&input.edit_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError(fx_core::Error::NotFound { entity: "edit_log", id: input.edit_id.clone() }))?;

    // Apply old_data back to the book
    let old = &log.old_data;
    fx_core::services::book_service::update_book(
        &state.pool,
        &log.book_id,
        old.get("title").and_then(|v| v.as_str()),
        old.get("description").and_then(|v| v.as_str()),
    ).await?;

    // Record revert as a new edit log entry
    let revert_id = tid();
    sqlx::query(
        "INSERT INTO book_edit_log (id, book_id, editor_did, old_data, new_data, summary) \
         VALUES ($1, $2, 'admin', $3, $4, $5)",
    )
    .bind(&revert_id)
    .bind(&log.book_id)
    .bind(&log.new_data)
    .bind(&log.old_data)
    .bind(format!("Reverted edit by {}", log.editor_handle.as_deref().unwrap_or(&log.editor_did)))
    .execute(&state.pool)
    .await?;

    Ok(Json(serde_json::json!({ "reverted": input.edit_id, "book_id": log.book_id })))
}

// --- Set default edition for a book ---

#[derive(serde::Deserialize)]
pub struct SetDefaultEditionInput {
    pub book_id: String,
    pub edition_id: String,
}

pub async fn admin_set_default_edition(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<SetDefaultEditionInput>,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query("UPDATE books SET default_edition_id = $1 WHERE id = $2")
        .bind(&input.edition_id)
        .bind(&input.book_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// --- Batch publish articles to a series (single pijul change) ---

#[derive(serde::Deserialize)]
pub struct BatchArticle {
    pub title: String,
    pub description: Option<String>,
    pub content: String,
    pub content_format: Option<String>,
    pub tags: Option<Vec<String>>,
    pub license: Option<String>,
    /// Path in the repo (e.g. "ch1/34DataFlowAnalysis.md"). If omitted, uses chapters/{tid}.{ext}.
    pub path: Option<String>,
}

/// A binary file to write into the repo (e.g. images).
#[derive(serde::Deserialize)]
pub struct BatchFile {
    /// Path in the repo (e.g. "ch1/img/foo.png")
    pub path: String,
    /// Base64-encoded content
    pub data: String,
}

#[derive(serde::Deserialize)]
pub struct AdminBatchPublishInput {
    pub as_handle: String,
    pub series_id: String,
    pub articles: Vec<BatchArticle>,
    /// Extra binary files (images etc) to write into the repo
    #[serde(default)]
    pub files: Vec<BatchFile>,
    pub lang: Option<String>,
}

pub async fn admin_batch_publish(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<AdminBatchPublishInput>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    use fx_core::content::ContentFormat;

    let did = platform_user_service::local_did(&input.as_handle);
    let lang = input.lang.as_deref().unwrap_or("en");

    // Get series pijul node
    let pijul_node_id: Option<String> = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&input.series_id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    let node_id = pijul_node_id.ok_or_else(|| {
        AppError(fx_core::Error::BadRequest("Series has no pijul repo".into()))
    })?;

    // Ensure pijul repo exists (may have been deleted)
    if let Err(e) = state.pijul.init_series_repo(&node_id) {
        tracing::warn!("failed to init series pijul repo: {e}");
    }

    let series_repo = state.pijul.series_repo_path(&node_id);

    let mut results = Vec::new();

    // Phase 0: Write extra binary files (images etc) into the repo
    for file in &input.files {
        let file_path = series_repo.join(&file.path);
        if let Some(parent) = file_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        use base64::Engine;
        let data = base64::engine::general_purpose::STANDARD.decode(&file.data)
            .map_err(|e| AppError(fx_core::Error::BadRequest(format!("invalid base64 for {}: {e}", file.path))))?;
        tokio::fs::write(&file_path, &data).await?;
    }
    if !input.files.is_empty() {
        tracing::info!("wrote {} extra files to series repo", input.files.len());
    }

    // Phase 1: Write all article files and create/update DB records (no pijul record yet)
    for item in &input.articles {
        let format = item.content_format.as_deref().unwrap_or("markdown");
        let content_format: ContentFormat = format.parse().unwrap_or(ContentFormat::Markdown);
        let src_ext = fx_renderer::format_extension(format);
        let hash = content_hash(&item.content);
        let license = item.license.as_deref().unwrap_or("CC-BY-SA-4.0");

        // Determine repo_path for this article (used as the stable matching key)
        let repo_path = item.path.clone();

        // Check if an article with this path already exists in the series
        let existing_uri = if let Some(ref path) = repo_path {
            series_service::find_article_by_repo_path(&state.pool, &input.series_id, path).await?
        } else {
            None
        };

        let (at_uri, is_update) = if let Some(uri) = existing_uri {
            (uri, true)
        } else {
            (format!("at://{}/{}/{}", did, fx_atproto::lexicon::ARTICLE, tid()), false)
        };

        let chapter_id = at_uri.rsplit('/').next().unwrap_or("unknown");

        // Write file to series repo — use custom path or default
        let chapter_path = if let Some(ref p) = repo_path {
            series_repo.join(p)
        } else {
            fx_core::meta::resolve_chapter_path(&series_repo, chapter_id, src_ext)
        };
        if let Some(parent) = chapter_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        tokio::fs::write(&chapter_path, &item.content).await?;

        // Render cache
        if content_format != ContentFormat::Html {
            if let Ok(rendered) = super::articles::render_content(format, &item.content, &series_repo) {
                let cache_dir = series_repo.join("cache");
                let _ = tokio::fs::create_dir_all(&cache_dir).await;
                let _ = tokio::fs::write(cache_dir.join(format!("{chapter_id}.html")), &rendered).await;
            }
        }

        let create = CreateArticle {
            title: item.title.clone(),
            description: item.description.clone(),
            content: item.content.clone(),
            content_format,
            lang: Some(lang.to_string()),
            license: Some(license.to_string()),
            tags: item.tags.clone().unwrap_or_default(),
            prereqs: vec![],
            series_id: Some(input.series_id.clone()),
            translation_of: None,
            category: Some("lecture".to_string()),
            book_id: None,
            edition_id: None,
            restricted: None,
            invites: vec![],
        };

        let article = if is_update {
            tracing::info!("updating existing article {at_uri}");
            article_service::update_article_batch(&state.pool, &at_uri, &create, &hash).await?
        } else {
            let article = article_service::create_article(
                &state.pool, &did, &at_uri, &create, &hash, None,
                default_visibility(true), ContentKind::Article, None,
            ).await?;
            series_service::add_series_article_with_path(
                &state.pool, &input.series_id, &at_uri, repo_path.as_deref(),
            ).await?;
            article
        };

        results.push(serde_json::json!({
            "at_uri": at_uri,
            "title": article.title,
            "updated": is_update,
        }));
    }

    // Phase 2: Single pijul record for all files
    match state.pijul_record_series(node_id.clone(), "Batch publish".into(), Some(did.clone())).await {
        Ok(Some((hash, _new_state))) => {
            tracing::info!("batch recorded change {hash} for series {}", input.series_id);
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("pijul batch record failed: {e}"),
    }

    Ok(Json(results))
}

// ── Tag aliases ─────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct AddAliasInput {
    pub tag_id: String,
    pub alias: String,
}

pub async fn admin_add_tag_alias(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<AddAliasInput>,
) -> ApiResult<StatusCode> {
    tag_service::add_alias(&state.pool, &input.alias, &input.tag_id).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct RemoveAliasInput {
    pub alias: String,
}

pub async fn admin_remove_tag_alias(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<RemoveAliasInput>,
) -> ApiResult<StatusCode> {
    tag_service::remove_alias(&state.pool, &input.alias).await?;
    Ok(StatusCode::OK)
}
