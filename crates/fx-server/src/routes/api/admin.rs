use axum::{
    Json,
    extract::{Path, State},
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

    let publish = super::articles::publish_article_content(
        &state, &at_uri, &did, "", &input.article.content, input.article.content_format,
        input.article.series_id.as_deref(), "Initial publish",
        super::articles::SummaryInput {
            user_source: input.article.summary.as_deref(),
        },
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
        &publish.summary_source, &publish.summary_html,
    ).await?;

    if let Some(ref sid) = input.article.series_id {
        let chapter_id = at_uri.rsplit('/').next().unwrap_or("unknown");
        let src_ext = fx_renderer::format_extension(input.article.content_format.as_str());
        let default_path = fx_core::meta::default_chapter_path(chapter_id, src_ext);
        series_service::add_series_article(&state.pool, sid, &at_uri, Some(&default_path)).await?;
    }

    let _ = article_service::auto_bookmark(&state.pool, &did, &at_uri).await;

    Ok((StatusCode::CREATED, Json(article)))
}

// --- Admin series management ---

#[derive(serde::Deserialize)]
pub struct AdminCreateSeriesInput {
    pub as_handle: String,
    pub title: String,
    pub summary: Option<String>,
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

    let desc_html = match input.summary.as_deref() {
        Some(d) if !d.is_empty() => crate::summary::render_summary_inline(
            "markdown", d, &state.pijul.series_repo_path(&node_id),
        ).unwrap_or_default(),
        _ => String::new(),
    };

    let row = series_service::create_series(
        &state.pool,
        &id,
        &input.title,
        input.summary.as_deref(),
        &desc_html,
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


    // Admin-linking an existing article into a series: article's source lives
    // in its own standalone pijul repo, so repo_path is None.
    series_service::add_series_article(&state.pool, &input.series_id, &input.article_uri, None).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Admin article update (bypass ownership check) ---

#[derive(serde::Deserialize)]
pub struct AdminUpdateArticleInput {
    pub uri: String,
    pub title: Option<String>,
    pub summary: Option<String>,
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
    if let Some(ref summary) = input.summary {
        let format = article_service::get_content_format(&state.pool, &input.uri).await?;
        let node_id = fx_core::util::uri_to_node_id(&input.uri);
        let repo_path = state.pijul.repo_path(&node_id);
        let summary_html = crate::summary::render_summary_inline(format.as_str(), summary, &repo_path)
            .unwrap_or_default();
        article_service::update_article_summary(&state.pool, &input.uri, summary, &summary_html).await?;
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


    tag_service::merge_tag(&state.pool, &input.from, &input.into, "admin").await?;
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

    let publish = super::articles::publish_article_content(
        &state, &at_uri, &did, "", &input.article.content, input.article.content_format,
        None, "Initial publish",
        super::articles::SummaryInput {
            user_source: input.article.summary.as_deref(),
        },
    ).await?;

    let hash = content_hash(&input.article.content);

    let article = article_service::create_article(
        &state.pool, &did, &at_uri, &input.article, &hash, None,
        default_visibility(true), ContentKind::Question, None,
        &publish.summary_source, &publish.summary_html,
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

    let publish = super::articles::publish_article_content(
        &state, &at_uri, &did, "", &input.article.content, input.article.content_format,
        None, "Initial publish",
        super::articles::SummaryInput {
            user_source: input.article.summary.as_deref(),
        },
    ).await?;

    let hash = content_hash(&input.article.content);

    let article = article_service::create_article(
        &state.pool, &did, &at_uri, &input.article, &hash, None,
        default_visibility(true), ContentKind::Answer, Some(&input.question_uri),
        &publish.summary_source, &publish.summary_html,
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

// --- Tag deletion review ---

pub async fn admin_list_tag_deletion_requests(
    State(state): State<AppState>,
    _admin: AdminAuth,
) -> ApiResult<Json<Vec<tag_service::TagDeletionRequest>>> {
    let rows = tag_service::list_pending_tag_deletions(&state.pool).await?;
    Ok(Json(rows))
}

#[derive(serde::Deserialize)]
pub struct TagDeletionReviewInput {
    pub request_id: String,
    pub note: Option<String>,
}

pub async fn admin_approve_tag_deletion(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<TagDeletionReviewInput>,
) -> ApiResult<StatusCode> {
    // AdminAuth doesn't carry a DID; record the system as reviewer for
    // audit. Admin is authenticated by shared secret.
    tag_service::approve_tag_deletion(&state.pool, &input.request_id, "admin", input.note.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn admin_reject_tag_deletion(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<TagDeletionReviewInput>,
) -> ApiResult<StatusCode> {
    tag_service::reject_tag_deletion(&state.pool, &input.request_id, "admin", input.note.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
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
    /// Manifest override. When absent, the title falls back to the markdown
    /// frontmatter's `title` field, then to the first `# ` / `= ` / `<h1>`
    /// heading, then to the file stem.
    #[serde(default)]
    pub title: Option<String>,
    /// Manifest override for the article summary. Falls back to the
    /// frontmatter's `description` when absent.
    #[serde(default)]
    pub summary: Option<String>,
    pub content: String,
    #[serde(default)]
    pub content_format: Option<String>,
    /// Manifest override for tags. When absent, markdown frontmatter's
    /// `teaches` list is used.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub license: Option<String>,
    /// Path in the repo (e.g. "ch1/34DataFlowAnalysis.md"). Required — the
    /// path is the authoritative identifier of a chapter within a series.
    pub path: String,
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
        let _ = fx_renderer::format_extension(format);
        let hash = content_hash(&item.content);
        let license = item.license.as_deref().unwrap_or("CC-BY-SA-4.0");

        // repo_path is the authoritative identifier for a chapter in a series.
        let repo_path = item.path.as_str();

        // Check if an article with this path already exists in the series
        let existing_uri = series_service::find_article_by_repo_path(
            &state.pool, &input.series_id, repo_path,
        ).await?;

        let (at_uri, is_update) = if let Some(uri) = existing_uri {
            (uri, true)
        } else {
            (format!("at://{}/{}/{}", did, fx_atproto::lexicon::ARTICLE, tid()), false)
        };

        let chapter_id = at_uri.rsplit('/').next().unwrap_or("unknown");

        let chapter_path = series_repo.join(repo_path);
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

        // Markdown frontmatter is authoritative for per-chapter metadata
        // when the manifest doesn't override it. Non-markdown formats fall
        // back to the author-supplied manifest fields plus first-heading
        // extraction for title.
        let (fm, body_for_heading) = if format == "markdown" {
            let (fm, body) = fx_core::meta::split_frontmatter(&item.content);
            (fm, body)
        } else {
            (fx_core::meta::Frontmatter::default(), item.content.as_str())
        };

        let title = item.title.clone()
            .or_else(|| fm.title.clone())
            .or_else(|| fx_core::meta::extract_first_heading(body_for_heading, format))
            .unwrap_or_else(|| {
                std::path::Path::new(repo_path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Untitled".to_string())
            });

        let resolved_tags = item.tags.clone().unwrap_or_else(|| fm.teaches.clone());
        let resolved_prereqs: Vec<fx_core::models::ArticlePrereq> = fm.prereqs.iter()
            .map(|p| fx_core::models::ArticlePrereq {
                tag_id: p.tag.clone(),
                prereq_type: match p.kind() {
                    "recommended" => fx_core::content::PrereqType::Recommended,
                    _ => fx_core::content::PrereqType::Required,
                },
            })
            .collect();

        let create = CreateArticle {
            title,
            summary: item.summary.clone().or_else(|| fm.description.clone()),
            content: item.content.clone(),
            content_format,
            lang: Some(fm.lang.clone().unwrap_or_else(|| lang.to_string())),
            license: Some(fm.license.clone().unwrap_or_else(|| license.to_string())),
            tags: resolved_tags,
            prereqs: resolved_prereqs,
            series_id: Some(input.series_id.clone()),
            translation_of: None,
            category: Some(fm.category.clone().unwrap_or_else(|| "lecture".to_string())),
            restricted: None,
            metadata: None,
            authors: vec![],
            invites: vec![],
            book_chapter_id: None,
            course_session_id: None,
        };

        let resolved_desc = create.summary.as_deref().unwrap_or("").to_string();
        let desc_html = crate::summary::render_summary_inline(
            create.content_format.as_str(), &resolved_desc,
            &state.pijul.repo_path(&node_id),
        ).unwrap_or_default();

        let article = if is_update {
            tracing::info!("updating existing article {at_uri}");
            article_service::update_article_batch(&state.pool, &at_uri, &create, &hash, &resolved_desc, &desc_html).await?
        } else {
            let article = article_service::create_article(
                &state.pool, &did, &at_uri, &create, &hash, None,
                default_visibility(true), ContentKind::Article, None,
                &resolved_desc, &desc_html,
            ).await?;
            series_service::add_series_article(
                &state.pool, &input.series_id, &at_uri, Some(repo_path),
            ).await?;
            article
        };

        results.push(serde_json::json!({
            "at_uri": at_uri,
            "title": article.title,
            "updated": is_update,
        }));
    }

    // Phase 2a: Sync meta.yaml to reflect the current chapter list (in DB
    // order, which mirrors how files were just written). Pijul is the source
    // of truth for series structure; DB is the indexed cache.
    write_series_meta_from_db(&state, &input.series_id, &series_repo).await;

    // Phase 2b: Single pijul record for all files
    match state.pijul_record_series(node_id.clone(), "Batch publish".into(), Some(did.clone())).await {
        Ok(Some((hash, _new_state))) => {
            tracing::info!("batch recorded change {hash} for series {}", input.series_id);
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("pijul batch record failed: {e}"),
    }

    Ok(Json(results))
}

/// Rebuild a series' DB index from its pijul repo. Walks the `chapters:`
/// list in meta.yaml, reads each file's YAML frontmatter (markdown) or first
/// heading (typst/html), and reconciles `series_articles` + `articles.title`
/// + `content_teaches` + `content_prereqs`.
///
/// This is the inverse of `write_series_meta_from_db`: on-disk pijul content
/// becomes the authority, DB is rewritten to match.
pub async fn admin_rebuild_series_index(
    State(state): State<AppState>,
    Path(series_id): Path<String>,
    _admin: AdminAuth,
) -> ApiResult<Json<serde_json::Value>> {
    let pijul_node_id: String = sqlx::query_scalar(
        "SELECT pijul_node_id FROM series WHERE id = $1",
    )
    .bind(&series_id)
    .fetch_optional(&state.pool)
    .await?
    .flatten()
    .ok_or_else(|| AppError(fx_core::Error::BadRequest("series has no pijul repo".into())))?;

    let series_repo = state.pijul.series_repo_path(&pijul_node_id);
    let meta = fx_core::meta::read_series_meta(&series_repo)
        .ok_or_else(|| AppError(fx_core::Error::BadRequest(
            "series has no meta.yaml".into(),
        )))?;

    // Existing repo_path → article_uri map.
    let existing: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT repo_path, article_uri FROM series_articles \
         WHERE series_id = $1 AND repo_path IS NOT NULL AND repo_path <> ''",
    )
    .bind(&series_id)
    .fetch_all(&state.pool)
    .await?;
    let mut by_path: std::collections::HashMap<String, String> =
        existing.into_iter().collect();

    let did: String = sqlx::query_scalar("SELECT created_by FROM series WHERE id = $1")
        .bind(&series_id)
        .fetch_one(&state.pool)
        .await?;

    let mut added = 0u64;
    let mut updated = 0u64;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (order_index, rel_path) in meta.chapters.iter().enumerate() {
        let full = series_repo.join(rel_path);
        let Ok(content) = tokio::fs::read_to_string(&full).await else {
            tracing::warn!("rebuild: missing chapter file {rel_path}");
            continue;
        };
        seen.insert(rel_path.clone());

        let ext = std::path::Path::new(rel_path)
            .extension().and_then(|e| e.to_str()).unwrap_or("md");
        let format = match ext {
            "typ" | "typst" => "typst",
            "html" | "htm" => "html",
            _ => "markdown",
        };

        let (fm, body) = if format == "markdown" {
            fx_core::meta::split_frontmatter(&content)
        } else {
            (fx_core::meta::Frontmatter::default(), content.as_str())
        };
        let title = fm.title.clone()
            .or_else(|| fx_core::meta::extract_first_heading(body, format))
            .unwrap_or_else(|| rel_path.clone());

        let at_uri = match by_path.remove(rel_path) {
            Some(uri) => {
                sqlx::query(
                    "UPDATE series_articles SET order_index = $1 \
                     WHERE series_id = $2 AND article_uri = $3",
                )
                .bind(order_index as i32).bind(&series_id).bind(&uri)
                .execute(&state.pool).await?;
                sqlx::query("UPDATE articles SET title = $1 WHERE at_uri = $2")
                    .bind(&title).bind(&uri).execute(&state.pool).await?;
                updated += 1;
                uri
            }
            None => {
                let new_uri = format!("at://{did}/{}/{}", fx_atproto::lexicon::ARTICLE, tid());
                let content_format: fx_core::content::ContentFormat =
                    format.parse().unwrap_or(fx_core::content::ContentFormat::Markdown);
                let hash = content_hash(&content);
                let license = fm.license.as_deref().unwrap_or("CC-BY-SA-4.0");
                let create = CreateArticle {
                    title: title.clone(),
                    summary: fm.description.clone(),
                    content: content.clone(),
                    content_format,
                    lang: fm.lang.clone(),
                    license: Some(license.to_string()),
                    tags: fm.teaches.clone(),
                    prereqs: vec![],
                    series_id: Some(series_id.clone()),
                    translation_of: None,
                    category: fm.category.clone().or(Some("lecture".into())),
                    restricted: None,
                    metadata: None,
                    authors: vec![],
                    invites: vec![],
                    book_chapter_id: None,
                    course_session_id: None,
                };
                let resolved_desc = create.summary.as_deref().unwrap_or("").to_string();
                let desc_html = crate::summary::render_summary_inline(
                    create.content_format.as_str(), &resolved_desc, &series_repo,
                ).unwrap_or_default();
                article_service::create_article(
                    &state.pool, &did, &new_uri, &create, &hash, None,
                    default_visibility(true), ContentKind::Article, None,
                    &resolved_desc, &desc_html,
                ).await?;
                series_service::add_series_article(
                    &state.pool, &series_id, &new_uri, Some(rel_path),
                ).await?;
                added += 1;
                new_uri
            }
        };

        // Rewrite teaches/prereqs to match frontmatter. Frontmatter may
        // carry tag_ids, label ids, or brand-new names — resolve_tag_id
        // normalizes all three to the canonical tag_id before linking.
        let _ = sqlx::query("DELETE FROM content_teaches WHERE content_uri = $1")
            .bind(&at_uri).execute(&state.pool).await;
        for input_ref in &fm.teaches {
            let mut conn = state.pool.acquire().await.expect("db conn");
            if let Ok(tag_id) = fx_core::services::tag_service::resolve_tag_id(&mut conn, input_ref, &did).await {
                let _ = sqlx::query(
                    "INSERT INTO content_teaches (content_uri, tag_id) \
                     VALUES ($1, $2) ON CONFLICT DO NOTHING",
                ).bind(&at_uri).bind(&tag_id).execute(&mut *conn).await;
            }
        }
        let _ = sqlx::query("DELETE FROM content_prereqs WHERE content_uri = $1")
            .bind(&at_uri).execute(&state.pool).await;
        for p in &fm.prereqs {
            let mut conn = state.pool.acquire().await.expect("db conn");
            if let Ok(tag_id) = fx_core::services::tag_service::resolve_tag_id(&mut conn, &p.tag, &did).await {
                let _ = sqlx::query(
                    "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) \
                     VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                )
                .bind(&at_uri).bind(&tag_id).bind(p.kind())
                .execute(&mut *conn).await;
            }
        }
    }

    // Anything left in `by_path` was in DB but is no longer listed in meta.yaml.
    let mut removed = 0u64;
    for (_rel, stale_uri) in by_path {
        sqlx::query(
            "DELETE FROM series_articles WHERE series_id = $1 AND article_uri = $2",
        )
        .bind(&series_id).bind(&stale_uri)
        .execute(&state.pool).await?;
        removed += 1;
    }

    // Also sync the series-level fields (title, etc.) from meta.yaml.
    sqlx::query("UPDATE series SET title = $1 WHERE id = $2 AND title <> $1")
        .bind(&meta.title).bind(&series_id).execute(&state.pool).await?;
    if let Some(desc) = &meta.description {
        sqlx::query("UPDATE series SET summary = $1 WHERE id = $2")
            .bind(desc).bind(&series_id).execute(&state.pool).await?;
    }
    if let Some(level) = meta.split_level {
        if (1..=6).contains(&level) {
            sqlx::query("UPDATE series SET split_level = $1 WHERE id = $2")
                .bind(level as i32).bind(&series_id).execute(&state.pool).await?;
        }
    }

    Ok(Json(serde_json::json!({
        "chapters_added":   added,
        "chapters_updated": updated,
        "chapters_removed": removed,
        "chapters_seen":    seen.len(),
    })))
}

/// Rebuild meta.yaml from the authoritative DB state of a series. Called
/// after any write operation that touches chapters (batch-publish, compile,
/// reorder…) so the repo-side metadata stays in sync.
async fn write_series_meta_from_db(
    state: &AppState,
    series_id: &str,
    series_repo: &std::path::Path,
) {
    // Series-level fields
    let row: Option<(String, Option<String>, Option<String>, Option<String>, Option<String>, i32)> =
        sqlx::query_as(
            "SELECT title, summary, long_description, lang, category, split_level \
             FROM series WHERE id = $1",
        )
        .bind(series_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
    let Some((title, summary, long_description, lang, category, split_level)) = row else {
        return;
    };

    // Topics (content_topics join)
    let topics: Vec<String> = sqlx::query_scalar(
        "SELECT tag_id FROM content_topics WHERE content_uri = $1 ORDER BY tag_id",
    )
    .bind(format!("series:{series_id}"))
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Chapter paths in order — only for file-based chapters (markdown etc.),
    // typst series derive chapters from main.typ so leave the list empty.
    let chapters: Vec<String> = sqlx::query_scalar(
        "SELECT repo_path FROM series_articles \
         WHERE series_id = $1 AND repo_path IS NOT NULL AND repo_path <> '' \
         ORDER BY order_index",
    )
    .bind(series_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let meta = fx_core::meta::SeriesMeta {
        title,
        description: summary,
        long_description,
        lang,
        category,
        topics,
        split_level: Some(split_level as u32),
        chapters,
        cover: None,
    };

    if let Err(e) = fx_core::meta::write_series_meta(series_repo, &meta) {
        tracing::warn!("failed to write meta.yaml for {series_id}: {e}");
    }
}

