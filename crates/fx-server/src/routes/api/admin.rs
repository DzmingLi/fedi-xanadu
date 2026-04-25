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
    let token = ""; // No PDS session for admin; blob upload synthesizes locally for did:local:*.

    if let Some(ref sid) = input.article.series_id {
        // Admin publish into a series: same unified-series shape as the
        // user-auth path. Series chapters don't get their own at_uri; the
        // source lives under the series's shared blob_cache dir.
        let series_repo_uri = series_service::series_repo_uri(&state.pool, sid).await?;
        let chapter_tid = fx_core::util::tid();
        let src_ext = fx_renderer::format_extension(input.article.content_format.as_str());
        let source_path = fx_core::meta::default_chapter_path(&chapter_tid, src_ext);

        let publish = super::articles::publish_article_blob_to(
            &state, &did, token, &input.article.content, input.article.content_format,
            super::articles::SummaryInput { user_source: input.article.summary.as_deref() },
            super::articles::PublishTarget::SeriesChapter {
                series_repo_uri: &series_repo_uri,
                chapter_path: &source_path,
            },
        ).await?;

        let hash = content_hash(&input.article.content);
        let article = article_service::create_series_chapter(
            &state.pool, &did, &series_repo_uri, &source_path,
            &input.article, &hash,
            default_visibility(true),
            &publish.summary_source, &publish.summary_html,
            publish.blob_manifest.clone(),
        ).await?;

        let _ = series_service::add_series_chapter(
            &state.pool, sid, &series_repo_uri, &source_path, Some(&chapter_tid),
        ).await?;

        // Admin has no user PDS session, so we skip the series-record merge.
        // Federation catches up when the author re-publishes through their
        // own session.
        return Ok((StatusCode::CREATED, Json(article)));
    }

    // Standalone article.
    let at_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::WORK, fx_core::util::tid());

    let publish = super::articles::publish_article_blob(
        &state, &at_uri, &did, token, &input.article.content, input.article.content_format,
        super::articles::SummaryInput { user_source: input.article.summary.as_deref() },
    ).await?;

    let hash = content_hash(&input.article.content);
    let translation_group = if let Some(ref source_uri) = input.article.translation_of {
        #[allow(deprecated)]
        Some(article_service::resolve_translation_group(&state.pool, source_uri).await?)
    } else {
        None
    };

    let article = article_service::create_article(
        &state.pool, &did, &at_uri, &input.article, &hash, translation_group,
        default_visibility(true), ContentKind::Article, None,
        &publish.summary_source, &publish.summary_html,
        publish.blob_manifest.clone(),
    ).await?;

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
        #[allow(deprecated)]
        Some(series_service::resolve_series_translation_group(&state.pool, source_id).await?)
    } else {
        None
    };

    let category = input.category.as_deref().unwrap_or("general");

    let desc_html = match input.summary.as_deref() {
        Some(d) if !d.is_empty() => crate::summary::render_summary_inline(
            "markdown", d, &state.blob_cache_path,
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
        let repo_path = state.blob_cache_path.join(&node_id);
        let summary_html = crate::summary::render_summary_inline(format.as_str(), summary, &repo_path)
            .unwrap_or_default();
        article_service::update_article_summary(&state.pool, &input.uri, summary, &summary_html).await?;
    }

    if let Some(content) = input.content.as_ref() {
        // Resolve current storage + author from DB, then re-publish through
        // the blob path. For did:local:* authors the blob is synthesized
        // locally (no PDS upload). For real-PDS authors, admin has no user
        // token so the PDS blob upload silently no-ops — local blob_cache + DB
        // are updated but the PDS record stays stale until the author
        // re-publishes via their own session.
        let (author_did, fmt_str): (String, String) = sqlx::query_as(
            "SELECT a.author_did, l.content_format::text \
             FROM articles a JOIN article_localizations l \
               ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
             WHERE l.at_uri = $1 LIMIT 1",
        )
        .bind(&input.uri)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| AppError(fx_core::Error::NotFound { entity: "article", id: input.uri.clone() }))?;
        let format = fmt_str.parse::<fx_core::content::ContentFormat>()
            .unwrap_or(fx_core::content::ContentFormat::Markdown);
        let _publish = super::articles::publish_article_blob(
            &state, &input.uri, &author_did, "", content, format,
            super::articles::SummaryInput { user_source: input.summary.as_deref() },
        ).await?;
        let hash = content_hash(content);
        // Update content_hash on the localization row directly.
        sqlx::query(
            "UPDATE article_localizations SET content_hash = $1, updated_at = NOW() WHERE at_uri = $2",
        )
        .bind(&hash).bind(&input.uri)
        .execute(&state.pool).await?;
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
        &article.author_did,
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
            &state.pool, &notif_id, &article.author_did, "system",
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
    let at_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::WORK, tid());

    // Questions are server-only — no PDS record, no blob bundle — so no
    // token issue here. We just skip the blob publish and record the DB row.
    let hash = content_hash(&input.article.content);

    let article = article_service::create_article(
        &state.pool, &did, &at_uri, &input.article, &hash, None,
        default_visibility(true), ContentKind::Question, None,
        "", "",
        None,
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
    let at_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::WORK, tid());

    // Q&A is server-only (no PDS record, no blob bundle), so the admin path
    // is safe to keep.
    let hash = content_hash(&input.article.content);

    let article = article_service::create_article(
        &state.pool, &did, &at_uri, &input.article, &hash, None,
        default_visibility(true), ContentKind::Answer, Some(&input.question_uri),
        "", "",
        None,
    ).await?;

    // Notify question author
    if question.author_did != did {
        let notif_id = tid();
        let _ = notification_service::create_notification(
            &state.pool, &notif_id, &question.author_did, &did,
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
                l.action, l.target_id, l.old_data, l.new_data, l.summary, l.created_at \
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

    let author_did = platform_user_service::local_did(&input.as_handle);
    let lang = input.lang.as_deref().unwrap_or("en");
    let series_repo_uri = series_service::series_repo_uri(&state.pool, &input.series_id).await?;
    let series_node_id = fx_core::util::uri_to_node_id(&series_repo_uri);
    let series_root = state.blob_cache_path.join(&series_node_id);
    tokio::fs::create_dir_all(&series_root).await?;

    // Stage extra binary files (images etc.) directly into the series's
    // shared blob_cache dir. They become part of the series `files[]` on
    // next publish.
    for file in &input.files {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let Ok(bytes) = STANDARD.decode(&file.data) else {
            tracing::warn!("batch-publish: invalid base64 for {}", file.path);
            continue;
        };
        let safe_path: String = file.path.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' { c } else { '_' })
            .collect();
        let safe_path = safe_path.trim_start_matches('.').trim_start_matches('/').to_string();
        if safe_path.is_empty() || safe_path.contains("..") { continue; }
        let dest = series_root.join(&safe_path);
        if let Some(parent) = dest.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&dest, &bytes).await;
    }

    let mut results = Vec::new();

    for item in &input.articles {
        let format = item.content_format.as_deref().unwrap_or("markdown");
        let content_format: ContentFormat = format.parse().unwrap_or(ContentFormat::Markdown);
        let hash = content_hash(&item.content);
        let license = item.license.as_deref().unwrap_or("CC-BY-SA-4.0");
        let source_path = item.path.as_str();

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
                std::path::Path::new(source_path)
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
            related: vec![],
            topics: vec![],
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

        // Publish chapter source into the series blob_cache under its path.
        let publish = super::articles::publish_article_blob_to(
            &state, &author_did, "", &item.content, content_format,
            super::articles::SummaryInput { user_source: create.summary.as_deref() },
            super::articles::PublishTarget::SeriesChapter {
                series_repo_uri: &series_repo_uri,
                chapter_path: source_path,
            },
        ).await?;

        // Idempotent: create_series_chapter UPSERTs by (repo_uri, source_path),
        // so re-running a batch with the same paths updates in place.
        let existing = sqlx::query_scalar::<_, Option<String>>(
            "SELECT heading_anchor FROM series_articles \
             WHERE series_id = $1 AND repo_uri = $2 AND source_path = $3",
        )
        .bind(&input.series_id).bind(&series_repo_uri).bind(source_path)
        .fetch_optional(&state.pool).await?.flatten();
        let is_update = existing.is_some();
        let anchor = existing.unwrap_or_else(|| {
            // Derive a stable anchor from the source path's file stem.
            std::path::Path::new(source_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("chapter")
                .to_string()
        });

        let article = article_service::create_series_chapter(
            &state.pool, &author_did, &series_repo_uri, source_path,
            &create, &hash,
            default_visibility(true),
            &publish.summary_source, &publish.summary_html,
            publish.blob_manifest.clone(),
        ).await?;

        let _ = series_service::add_series_chapter(
            &state.pool, &input.series_id, &series_repo_uri, source_path, Some(&anchor),
        ).await?;

        results.push(serde_json::json!({
            "source_path": source_path,
            "title": article.title,
            "updated": is_update,
        }));
    }

    Ok(Json(results))
}

/// Rebuild a series' DB index from its pijul repo. Previously walked the
/// `chapters:` list in meta.yaml. With pijul gone there's no on-disk series
/// repo to rebuild from — per-chapter manifests on the DB ARE the source of
/// truth. Returns BadRequest until a new rebuild-from-PDS path is designed.
pub async fn admin_rebuild_series_index(
    State(_state): State<AppState>,
    Path(_series_id): Path<String>,
    _admin: AdminAuth,
) -> ApiResult<Json<serde_json::Value>> {
    Err(AppError(fx_core::Error::BadRequest(
        "admin_rebuild_series_index is disabled in the blob storage model: \
         series meta.yaml no longer exists, and chapters are addressed by \
         per-article blob manifest. Rebuild-from-PDS path pending design.".into(),
    )))
}

/// GET /api/admin/consistency/pijul — previously reported mismatches
/// between the `articles`/`series` tables and the on-disk pijul repo store.
/// Pijul storage is gone, so this endpoint is a no-op. Kept for wire-format
/// stability (CLI may still call it); returns an empty report shape.
pub async fn check_pijul_consistency(
    State(_state): State<AppState>,
    _admin: AdminAuth,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "article_missing_repo": [],
        "series_missing_repo": [],
        "orphan_dirs": [],
        "total_dirs_scanned": 0,
    })))
}

#[derive(serde::Deserialize)]
pub struct MigratePijulInput {
    /// Filesystem path of the legacy pijul scratch tree
    /// (e.g. /var/lib/nightboat/pijul-store). Each article/series had its files
    /// under {pijul_scratch_path}/{uri_to_node_id(repo_uri)}/.
    pub pijul_scratch_path: String,
    /// Dry-run: compute what would change, don't write anything.
    #[serde(default)]
    pub dry_run: bool,
}

/// One-shot migration: walk every `article_localizations` row with
/// `content_storage = 'pijul'`, read its source files from the legacy pijul
/// scratch tree, synthesize a blob manifest (local CIDs via blake3), populate
/// `blob_cache_path/{node_id}/`, and flip the row to `content_storage='blob'`.
/// Migrates a legacy pijul scratch tree into the blob model.
///
/// Two shapes of `repo_uri` are present in the DB after the earlier schema
/// rewrite:
///   1. Standalone article: exactly one `article_localizations` row per
///      repo_uri. The whole scratch dir belongs to this one article.
///   2. Series: many `article_localizations` rows share the same repo_uri
///      (which IS the series's `at.nightbo.work` at-uri). The scratch dir
///      holds the entire series bundle — chapter sources, shared bibliography,
///      figures, etc.
///
/// For standalone articles we preserve the existing behavior: each source
/// file's subtree under scratch_dir becomes the article's blob manifest.
///
/// For series: we copy every file under `{pijul_scratch}/{series_node_id}/`
/// (minus derived dirs: .pijul, cache, _rendered) into
/// `{blob_cache}/{series_node_id}/` and synthesize the series's
/// `chapters[]` + `files[]` shape directly on each chapter's DB row. No
/// at_uri is minted — chapters stay at_uri=NULL by design. No PDS write
/// (admin has no user session; federation happens when the author
/// re-publishes).
///
/// Idempotent: rows already at `content_storage='blob'` are skipped.
pub async fn admin_migrate_pijul_to_blob(
    State(state): State<AppState>,
    _admin: AdminAuth,
    Json(input): Json<MigratePijulInput>,
) -> ApiResult<Json<serde_json::Value>> {
    use std::path::{Path, PathBuf};

    fn is_derived(path: &Path) -> bool {
        let s = path.to_string_lossy();
        s.starts_with(".pijul/")
            || s.starts_with("cache/")
            || s.starts_with("_rendered/")
            || s == ".pijul.toml"
            || s == "content.html"
            || s == "summary.html"
            || s.ends_with(".swp")
    }

    fn walk_files(root: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(d) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&d) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if matches!(name, ".pijul" | "cache" | "_rendered") { continue; }
                    stack.push(p);
                } else {
                    out.push(p);
                }
            }
        }
        out
    }

    fn mime_for(rel: &Path) -> String {
        let ext = rel.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "md"   => "text/markdown",
            "typ"  => "text/x-typst",
            "html" => "text/html",
            "bib"  => "application/x-bibtex",
            "png"  => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif"  => "image/gif",
            "svg"  => "image/svg+xml",
            "pdf"  => "application/pdf",
            "txt"  => "text/plain",
            "yaml" | "yml" => "application/yaml",
            "json" => "application/json",
            _      => "application/octet-stream",
        }.to_string()
    }

    let base = std::path::PathBuf::from(&input.pijul_scratch_path);
    if !base.is_dir() {
        return Err(AppError(fx_core::Error::BadRequest(
            format!("pijul_scratch_path {:?} is not a directory", base),
        )));
    }

    // Group pijul-backed rows by repo_uri: standalone → exactly one, series → many.
    #[derive(Debug)]
    struct RowInfo {
        source_path: String,
        lang: String,
        at_uri: Option<String>,
        content_format: String,
    }
    let rows: Vec<(String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT l.repo_uri, l.source_path, l.lang, l.at_uri, l.content_format::text \
         FROM article_localizations l \
         WHERE l.content_storage = 'pijul'",
    )
    .fetch_all(&state.pool)
    .await?;

    let mut groups: std::collections::BTreeMap<String, Vec<RowInfo>> =
        std::collections::BTreeMap::new();
    for (repo_uri, source_path, lang, at_uri, content_format) in rows {
        groups.entry(repo_uri).or_default().push(RowInfo {
            source_path, lang, at_uri, content_format,
        });
    }

    let mut migrated_articles = 0usize;
    let mut migrated_chapters = 0usize;
    let mut series_migrated = 0usize;
    let mut skipped_missing = Vec::<serde_json::Value>::new();
    let mut errors = Vec::<serde_json::Value>::new();

    for (repo_uri, rows) in groups {
        let repo_node_id = fx_core::util::uri_to_node_id(&repo_uri);

        // The "is this a series?" heuristic: repo_uri is under the legacy
        // series NSID (the article_repos_rewrite migration backfilled chapter
        // rows with `at://{did}/at.nightbo.series/{id}` synthetic uris) OR
        // multiple rows share the repo_uri (multi-chapter set).
        let is_series = repo_uri.contains("/at.nightbo.series/")
            || repo_uri.contains(&format!("/{}/", fx_atproto::lexicon::WORK))
            || rows.len() > 1;

        // Legacy on-disk naming: series pijul repos lived at `series_{rkey}/`
        // (NOT at `uri_to_node_id` of the synthesized series at-uri).
        // Standalone articles always lived at `uri_to_node_id(at_uri)/`.
        let scratch_dir = if is_series {
            let rkey = repo_uri.rsplit('/').next().unwrap_or("");
            base.join(format!("series_{rkey}"))
        } else {
            base.join(&repo_node_id)
        };

        if is_series {
            // Copy the entire scratch tree into {blob_cache}/{series_node_id}/.
            if !scratch_dir.is_dir() {
                skipped_missing.push(serde_json::json!({
                    "repo_uri": repo_uri,
                    "reason": format!("series scratch dir not found at {:?}", scratch_dir),
                }));
                continue;
            }
            let target_dir = state.blob_cache_path.join(&repo_node_id);
            if !input.dry_run {
                if let Err(e) = tokio::fs::create_dir_all(&target_dir).await {
                    errors.push(serde_json::json!({
                        "repo_uri": repo_uri,
                        "stage": "mkdir-series", "error": e.to_string(),
                    }));
                    continue;
                }
            }

            // Build the canonical `files[]` (union of every source file in
            // the bundle). Each chapter's manifest points at this same set.
            let mut series_files = Vec::<serde_json::Value>::new();
            for abs in walk_files(&scratch_dir) {
                let rel = match abs.strip_prefix(&scratch_dir) {
                    Ok(r) => r.to_path_buf(),
                    Err(_) => continue,
                };
                if is_derived(&rel) { continue; }

                let bytes = match std::fs::read(&abs) {
                    Ok(b) => b,
                    Err(e) => {
                        errors.push(serde_json::json!({
                            "repo_uri": repo_uri,
                            "stage": "read", "file": rel.to_string_lossy(),
                            "error": e.to_string(),
                        }));
                        continue;
                    }
                };
                let cid = format!("local:{}", blake3::hash(&bytes).to_hex());
                let mime = mime_for(&rel);
                let size = bytes.len() as u64;

                if !input.dry_run {
                    let dest = target_dir.join(&rel);
                    if let Some(parent) = dest.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = std::fs::write(&dest, &bytes) {
                        errors.push(serde_json::json!({
                            "repo_uri": repo_uri,
                            "stage": "write", "file": rel.to_string_lossy(),
                            "error": e.to_string(),
                        }));
                        continue;
                    }
                }

                series_files.push(serde_json::json!({
                    "path": rel.to_string_lossy(),
                    "cid": cid,
                    "size": size,
                    "mime": mime,
                }));
            }

            // Flip each chapter row to blob storage with the shared manifest
            // pointing at its own entry. at_uri stays NULL.
            for r in &rows {
                let manifest = serde_json::json!({
                    "entry": r.source_path,
                    "files": series_files,
                    "pds_url": state.pds_url,
                });
                if !input.dry_run {
                    if let Err(e) = sqlx::query(
                        "UPDATE article_localizations \
                         SET content_storage = 'blob', content_manifest = $1, updated_at = NOW() \
                         WHERE repo_uri = $2 AND source_path = $3 AND lang = $4",
                    )
                    .bind(&manifest)
                    .bind(&repo_uri)
                    .bind(&r.source_path)
                    .bind(&r.lang)
                    .execute(&state.pool).await
                    {
                        errors.push(serde_json::json!({
                            "repo_uri": repo_uri, "source_path": r.source_path,
                            "stage": "db-update-chapter", "error": e.to_string(),
                        }));
                        continue;
                    }
                }
                migrated_chapters += 1;
                let _ = r.at_uri; // chapters never had one; nothing to mint.
            }

            // Also stamp heading_anchor on series_articles rows that don't
            // have one — the anchor is the chapter's source_path file stem
            // for stability across re-publishes.
            if !input.dry_run {
                for r in &rows {
                    let anchor_stem = std::path::Path::new(&r.source_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("chapter")
                        .to_string();
                    let _ = sqlx::query(
                        "UPDATE series_articles SET heading_anchor = COALESCE(heading_anchor, $1) \
                         WHERE repo_uri = $2 AND source_path = $3",
                    )
                    .bind(&anchor_stem).bind(&repo_uri).bind(&r.source_path)
                    .execute(&state.pool).await;
                }
            }

            series_migrated += 1;
        } else {
            // Standalone article: one row, per-article scratch dir with its
            // own files tree.
            let r = &rows[0];
            let at_uri = match r.at_uri.clone() {
                Some(u) => u,
                None => {
                    errors.push(serde_json::json!({
                        "repo_uri": repo_uri,
                        "source_path": r.source_path,
                        "stage": "no-at-uri", "error":
                            "standalone article has no at_uri; cannot migrate without one",
                    }));
                    continue;
                }
            };

            let source_file = scratch_dir.join(&r.source_path);
            if !source_file.is_file() {
                skipped_missing.push(serde_json::json!({
                    "repo_uri": repo_uri,
                    "source_path": r.source_path,
                    "reason": format!("source not found at {:?}", source_file),
                }));
                continue;
            }

            let target_node_id = fx_core::util::uri_to_node_id(&at_uri);
            let target_dir = state.blob_cache_path.join(&target_node_id);
            if !input.dry_run {
                if let Err(e) = tokio::fs::create_dir_all(&target_dir).await {
                    errors.push(serde_json::json!({
                        "repo_uri": repo_uri, "source_path": r.source_path,
                        "stage": "mkdir", "error": e.to_string(),
                    }));
                    continue;
                }
            }

            let src_ext = fx_renderer::format_extension(&r.content_format);
            let entry_rel = source_file.strip_prefix(&scratch_dir)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| format!("content.{src_ext}"));

            let mut manifest_files = Vec::<serde_json::Value>::new();
            for abs in walk_files(&scratch_dir) {
                let rel = match abs.strip_prefix(&scratch_dir) {
                    Ok(r) => r.to_path_buf(),
                    Err(_) => continue,
                };
                if is_derived(&rel) { continue; }

                let bytes = match std::fs::read(&abs) {
                    Ok(b) => b,
                    Err(e) => {
                        errors.push(serde_json::json!({
                            "repo_uri": repo_uri, "source_path": r.source_path,
                            "stage": "read", "file": rel.to_string_lossy(),
                            "error": e.to_string(),
                        }));
                        continue;
                    }
                };
                let cid = format!("local:{}", blake3::hash(&bytes).to_hex());
                let mime = mime_for(&rel);
                let size = bytes.len() as u64;

                if !input.dry_run {
                    let dest = target_dir.join(&rel);
                    if let Some(parent) = dest.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = std::fs::write(&dest, &bytes) {
                        errors.push(serde_json::json!({
                            "repo_uri": repo_uri, "source_path": r.source_path,
                            "stage": "write", "file": rel.to_string_lossy(),
                            "error": e.to_string(),
                        }));
                        continue;
                    }
                }

                manifest_files.push(serde_json::json!({
                    "path": rel.to_string_lossy(),
                    "cid": cid,
                    "size": size,
                    "mime": mime,
                }));
            }

            let manifest = serde_json::json!({
                "entry": entry_rel,
                "files": manifest_files,
                "pds_url": state.pds_url,
            });
            if !input.dry_run {
                if let Err(e) = sqlx::query(
                    "UPDATE article_localizations \
                     SET content_storage = 'blob', content_manifest = $1, updated_at = NOW() \
                     WHERE at_uri = $2",
                )
                .bind(&manifest).bind(&at_uri)
                .execute(&state.pool).await
                {
                    errors.push(serde_json::json!({
                        "repo_uri": repo_uri, "source_path": r.source_path,
                        "stage": "db-update", "error": e.to_string(),
                    }));
                    continue;
                }
            }
            migrated_articles += 1;
        }
    }

    Ok(Json(serde_json::json!({
        "dry_run": input.dry_run,
        "migrated_articles": migrated_articles,
        "migrated_chapters": migrated_chapters,
        "series_migrated": series_migrated,
        "skipped_missing": skipped_missing,
        "errors": errors,
    })))
}

