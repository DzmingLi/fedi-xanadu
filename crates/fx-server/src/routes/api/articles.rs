use axum::{
    Json,
    extract::{Multipart, Query, State},
    http::StatusCode,
};
use fx_core::models::*;
use fx_core::region::default_visibility;
use fx_core::services::{article_service, notification_service};
use fx_core::validation::validate_create_article;

use crate::error::{AppError, ApiResult, require_owner};
use crate::state::AppState;
use super::{WriteAuth, UriQuery, content_hash, tid, uri_to_node_id, pds_session, now_rfc3339, log_pds_error};

#[derive(serde::Deserialize)]
pub struct ListArticlesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_articles(
    State(state): State<AppState>,
    Query(q): Query<ListArticlesQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let offset = q.offset.unwrap_or(0).max(0);
    let articles = article_service::list_articles(&state.pool, state.instance_mode, limit, offset).await?;
    Ok(Json(articles))
}

pub async fn get_article(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Article>> {
    let article = article_service::get_article(&state.pool, state.instance_mode, &uri).await?;
    Ok(Json(article))
}

pub async fn get_article_content(
    State(state): State<AppState>,
    super::MaybeAuth(user): super::MaybeAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<ArticleContent>> {
    let has_access = article_service::check_content_access(
        &state.pool, &uri, user.as_ref().map(|u| u.did.as_str()),
    ).await?;
    if !has_access {
        return Err(AppError(fx_core::Error::Forbidden { action: "view restricted article" }));
    }

    let format = article_service::get_content_format(&state.pool, &uri).await?;

    let node_id = uri_to_node_id(&uri);
    let repo = state.pijul.repo_path(&node_id);

    let src_ext = match format.as_str() {
        "markdown" => "md",
        "html" => "html",
        _ => "typ",
    };
    let src_path = repo.join(format!("content.{src_ext}"));
    let html_path = repo.join("content.html");

    let source = match tokio::fs::read_to_string(&src_path).await {
        Ok(s) => s,
        Err(_) => {
            tokio::fs::read_to_string(repo.join("content.typ"))
                .await
                .map_err(|_| AppError(fx_core::Error::NotFound {
                    entity: "content",
                    id: uri.clone(),
                }))?
        }
    };

    let html = if format == "html" {
        source.clone()
    } else if is_cached_fresh(&html_path, &src_path).await {
        tokio::fs::read_to_string(&html_path).await?
    } else {
        let rendered = render_content(&format, &source, &repo)?;
        let _ = tokio::fs::write(&html_path, &rendered).await;
        rendered
    };

    Ok(Json(ArticleContent { source, html }))
}

async fn is_cached_fresh(cache: &std::path::Path, source: &std::path::Path) -> bool {
    let (cache_meta, source_meta) = tokio::join!(
        tokio::fs::metadata(cache),
        tokio::fs::metadata(source),
    );
    let (Ok(c), Ok(s)) = (cache_meta, source_meta) else { return false };
    let (Ok(c_mod), Ok(s_mod)) = (c.modified(), s.modified()) else { return false };
    c_mod >= s_mod
}

pub(super) fn render_content(format: &str, source: &str, repo_path: &std::path::Path) -> Result<String, AppError> {
    match format {
        "markdown" => fx_render::render_markdown_to_html(source)
            .map_err(|e| { tracing::warn!("render error: {e}"); AppError(fx_core::Error::Render(e.to_string())) }),
        _ => fx_render::render_typst_to_html_with_images(source, repo_path)
            .map_err(|e| { tracing::warn!("render error: {e}"); AppError(fx_core::Error::Render(e.to_string())) }),
    }
}

pub async fn get_article_prereqs(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<ArticlePrereqRow>>> {
    let prereqs = article_service::get_article_prereqs(&state.pool, &uri).await?;
    Ok(Json(prereqs))
}

pub async fn get_article_forks(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<ForkWithTitle>>> {
    let forks = article_service::get_article_forks(&state.pool, &uri).await?;
    Ok(Json(forks))
}

pub async fn create_article(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateArticle>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    validate_create_article(&input)?;

    let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::ARTICLE, tid());

    let node_id = uri_to_node_id(&at_uri);
    state.pijul.init_repo(&node_id)
        .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;

    let repo_path = state.pijul.repo_path(&node_id);
    let src_ext = match input.content_format.as_str() {
        "markdown" => "md",
        "html" => "html",
        _ => "typ",
    };
    tokio::fs::write(repo_path.join(format!("content.{src_ext}")), &input.content).await?;

    if input.content_format != "html" {
        let rendered_html = render_content(&input.content_format, &input.content, &repo_path)?;
        let _ = tokio::fs::write(repo_path.join("content.html"), &rendered_html).await;
    }

    if let Err(e) = state.pijul.record(&node_id, "Initial publish") {
        tracing::warn!("pijul record failed for {node_id}: {e}");
    }

    let hash = content_hash(&input.content);

    let translation_group = if let Some(ref source_uri) = input.translation_of {
        Some(article_service::resolve_translation_group(&state.pool, source_uri).await?)
    } else {
        None
    };

    let article = article_service::create_article(
        &state.pool, &user.did, &at_uri, &input, &hash, translation_group,
        default_visibility(user.phone_verified), "article", None,
    ).await?;

    // Skip PDS sync for restricted articles — content stays server-local only
    if !input.restricted.unwrap_or(false) {
        if let Some(pds) = pds_session(&state.pool, &user.token).await {
            let record = serde_json::json!({
                "$type": fx_atproto::lexicon::ARTICLE,
                "title": input.title,
                "description": input.description.as_deref().unwrap_or(""),
                "contentFormat": input.content_format,
                "tags": input.tags,
                "createdAt": now_rfc3339(),
            });
            if let Err(e) = state.at_client.create_record(
                &pds.pds_url, &pds.access_jwt,
                &fx_atproto::client::CreateRecordInput {
                    repo: pds.did,
                    collection: fx_atproto::lexicon::ARTICLE.to_string(),
                    record,
                    rkey: None,
                },
            ).await {
                log_pds_error("create article", e);
            }
        }
    }

    let _ = article_service::auto_bookmark(&state.pool, &user.did, &at_uri).await;

    Ok((StatusCode::CREATED, Json(article)))
}

// --- Full article page data (single request) ---

#[derive(serde::Serialize)]
pub struct ArticleFullResponse {
    article: Article,
    content: ArticleContent,
    prereqs: Vec<ArticlePrereqRow>,
    forks: Vec<ForkWithTitle>,
    votes: ArticleVoteSummary,
    series_context: Vec<fx_core::services::series_service::SeriesContextItem>,
    translations: Vec<Article>,
    my_vote: i32,
    is_bookmarked: bool,
    learned: bool,
    access_denied: bool,
}

#[derive(serde::Serialize)]
struct ArticleVoteSummary {
    score: i64,
    upvotes: i64,
    downvotes: i64,
}

pub async fn get_article_full(
    State(state): State<AppState>,
    super::MaybeAuth(user): super::MaybeAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<ArticleFullResponse>> {
    use fx_core::services::{vote_service, bookmark_service, series_service, learned_service};

    let mode = state.instance_mode;
    let (article, prereqs, forks, vote_summary, series_ctx, translations) = tokio::try_join!(
        article_service::get_article(&state.pool, mode, &uri),
        article_service::get_article_prereqs(&state.pool, &uri),
        article_service::get_article_forks(&state.pool, &uri),
        vote_service::get_vote_summary(&state.pool, &uri),
        series_service::get_series_context(&state.pool, &uri),
        article_service::get_translations(&state.pool, mode, &uri),
    ).map_err(AppError)?;

    // Access control check
    let has_access = article_service::check_content_access(
        &state.pool, &uri, user.as_ref().map(|u| u.did.as_str()),
    ).await?;

    let content = if has_access {
        let node_id = uri_to_node_id(&uri);
        let repo = state.pijul.repo_path(&node_id);
        let src_ext = match article.content_format.as_str() {
            "markdown" => "md",
            "html" => "html",
            _ => "typ",
        };
        let src_path = repo.join(format!("content.{src_ext}"));
        let html_path = repo.join("content.html");

        let source = match tokio::fs::read_to_string(&src_path).await {
            Ok(s) => s,
            Err(_) => tokio::fs::read_to_string(repo.join("content.typ"))
                .await
                .map_err(|_| AppError(fx_core::Error::NotFound { entity: "content", id: uri.clone() }))?,
        };
        let html = if article.content_format == "html" {
            source.clone()
        } else if is_cached_fresh(&html_path, &src_path).await {
            tokio::fs::read_to_string(&html_path).await?
        } else {
            let rendered = render_content(&article.content_format, &source, &repo)?;
            let _ = tokio::fs::write(&html_path, &rendered).await;
            rendered
        };
        ArticleContent { source, html }
    } else {
        ArticleContent { source: String::new(), html: String::new() }
    };

    let (my_vote, is_bookmarked, learned) = if let Some(ref u) = user {
        let (mv, bk, lr) = tokio::join!(
            vote_service::get_my_vote(&state.pool, &uri, &u.did),
            bookmark_service::list_bookmarks(&state.pool, &u.did),
            learned_service::is_learned(&state.pool, &u.did, &uri),
        );
        (
            mv.unwrap_or(0),
            bk.map(|bks| bks.iter().any(|b| b.article_uri == uri)).unwrap_or(false),
            lr.unwrap_or(false),
        )
    } else {
        (0, false, false)
    };

    Ok(Json(ArticleFullResponse {
        article,
        content,
        prereqs,
        forks,
        votes: ArticleVoteSummary {
            score: vote_summary.score,
            upvotes: vote_summary.upvotes,
            downvotes: vote_summary.downvotes,
        },
        series_context: series_ctx,
        translations,
        my_vote,
        is_bookmarked,
        learned,
        access_denied: !has_access,
    }))
}

// --- Bulk article metadata ---

#[derive(serde::Deserialize)]
pub struct BulkLimitQuery {
    pub limit: Option<i64>,
}

pub async fn get_all_article_teaches(
    State(state): State<AppState>,
    Query(q): Query<BulkLimitQuery>,
) -> ApiResult<Json<Vec<article_service::ContentTeachRow>>> {
    let limit = q.limit.unwrap_or(10_000).clamp(1, 50_000);
    let rows = article_service::get_all_article_teaches(&state.pool, limit).await?;
    Ok(Json(rows))
}

pub async fn get_all_article_prereqs(
    State(state): State<AppState>,
    Query(q): Query<BulkLimitQuery>,
) -> ApiResult<Json<Vec<article_service::ContentPrereqBulkRow>>> {
    let limit = q.limit.unwrap_or(10_000).clamp(1, 50_000);
    let rows = article_service::get_all_article_prereqs(&state.pool, limit).await?;
    Ok(Json(rows))
}

#[derive(serde::Deserialize)]
pub struct TagArticlesQuery {
    pub tag_id: String,
    pub limit: Option<i64>,
}

pub async fn get_articles_by_tag(
    State(state): State<AppState>,
    Query(q): Query<TagArticlesQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let articles = article_service::get_articles_by_tag(&state.pool, state.instance_mode, &q.tag_id, limit).await?;
    Ok(Json(articles))
}

#[derive(serde::Deserialize)]
pub struct DidArticlesQuery {
    pub did: String,
    pub limit: Option<i64>,
}

pub async fn get_articles_by_did(
    State(state): State<AppState>,
    Query(q): Query<DidArticlesQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let articles = article_service::get_articles_by_did(&state.pool, state.instance_mode, &q.did, limit).await?;
    Ok(Json(articles))
}

pub async fn get_translations(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let articles = article_service::get_translations(&state.pool, state.instance_mode, &uri).await?;
    Ok(Json(articles))
}

// --- Fork ---

pub async fn fork_article(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ForkArticleInput>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    if let Err(e) = fx_core::validation::validate_at_uri(&input.uri) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    let source = article_service::get_article(&state.pool, state.instance_mode, &input.uri).await?;

    if source.license == "All-Rights-Reserved" {
        return Err(AppError(fx_core::Error::Forbidden { action: "fork proprietary article" }));
    }
    if source.restricted {
        return Err(AppError(fx_core::Error::Forbidden { action: "fork restricted article" }));
    }

    let fork_at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::ARTICLE, tid());
    let fork_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::FORK, tid());

    let source_node_id = uri_to_node_id(&input.uri);
    let fork_node_id = uri_to_node_id(&fork_at_uri);
    state.pijul.fork(&source_node_id, &fork_node_id)
        .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;

    let article = article_service::create_fork_record(
        &state.pool, &fork_uri, &input.uri, &fork_at_uri, &user.did, &source,
        default_visibility(user.phone_verified),
    ).await?;

    if let Some(pds) = pds_session(&state.pool, &user.token).await {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::FORK,
            "source": input.uri,
            "fork": fork_at_uri,
            "createdAt": now_rfc3339(),
        });
        if let Err(e) = state.at_client.create_record(
            &pds.pds_url, &pds.access_jwt,
            &fx_atproto::client::CreateRecordInput {
                repo: pds.did,
                collection: fx_atproto::lexicon::FORK.to_string(),
                record,
                rkey: None,
            },
        ).await {
            log_pds_error("create fork", e);
        }
    }

    if let Err(e) = notification_service::create_notification(
        &state.pool, &tid(), &source.did, &user.did,
        "article_fork", Some(&input.uri), Some(&fork_at_uri),
    ).await {
        tracing::warn!("notification failed: {e}");
    }

    Ok((StatusCode::CREATED, Json(article)))
}

#[derive(serde::Deserialize)]
pub struct ForkArticleInput {
    uri: String,
}

// --- Image upload ---

const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
const ALLOWED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "svg", "webp"];

pub async fn upload_image(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    mut multipart: Multipart,
) -> ApiResult<Json<ImageUploadResponse>> {
    let mut article_uri: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError(fx_core::Error::BadRequest(format!("Multipart error: {e}")))
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "article_uri" => {
                article_uri = Some(field.text().await.map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?);
            }
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                file_data = Some(field.bytes().await.map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?.to_vec());
            }
            _ => {}
        }
    }

    let uri = article_uri.ok_or(AppError(fx_core::Error::BadRequest("Missing article_uri".into())))?;
    let original_name = file_name.ok_or(AppError(fx_core::Error::BadRequest("Missing file".into())))?;
    let data = file_data.ok_or(AppError(fx_core::Error::BadRequest("Missing file data".into())))?;

    if data.len() > MAX_IMAGE_SIZE {
        return Err(AppError(fx_core::Error::BadRequest("File too large (max 10MB)".into())));
    }

    let owner = article_service::get_article_owner(&state.pool, &uri).await?;
    require_owner(Some(&owner), &user.did)?;

    let ext = std::path::Path::new(&original_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest(format!("Unsupported file type: {ext}"))));
    }

    let safe_name: String = original_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect();
    let safe_name = safe_name.trim_start_matches('.').to_string();
    if safe_name.is_empty() || safe_name == "content.typ" || safe_name == "content.md" || safe_name == "content.html" {
        return Err(AppError(fx_core::Error::BadRequest("invalid file name".into())));
    }

    let node_id = uri_to_node_id(&uri);
    let repo_path = state.pijul.repo_path(&node_id);
    let dest = repo_path.join(&safe_name);

    tokio::fs::write(&dest, &data).await?;

    let _ = tokio::fs::remove_file(repo_path.join("content.html")).await;

    if let Err(e) = state.pijul.record(&node_id, &format!("Add image: {safe_name}")) {
        tracing::warn!("pijul record failed for {node_id}: {e}");
    }

    Ok(Json(ImageUploadResponse { filename: safe_name }))
}

#[derive(serde::Serialize)]
pub struct ImageUploadResponse {
    pub filename: String,
}

// --- Serve article images ---

#[derive(serde::Deserialize)]
pub struct ImageQuery {
    pub uri: String,
    pub name: String,
}

pub async fn get_image(
    State(state): State<AppState>,
    Query(q): Query<ImageQuery>,
) -> ApiResult<(axum::http::HeaderMap, Vec<u8>)> {
    let node_id = uri_to_node_id(&q.uri);
    let repo_path = state.pijul.repo_path(&node_id);

    let name = std::path::Path::new(&q.name)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(AppError(fx_core::Error::BadRequest("invalid file name".into())))?;

    let path = repo_path.join(name);
    let data = tokio::fs::read(&path).await.map_err(|_| {
        AppError(fx_core::Error::NotFound { entity: "image", id: name.to_string() })
    })?;

    let content_type = match std::path::Path::new(name).extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    };

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("content-type", content_type.parse().expect("valid content-type"));
    headers.insert("cache-control", "public, max-age=86400".parse().expect("valid cache-control"));
    Ok((headers, data))
}

// --- Update article ---

#[derive(serde::Deserialize)]
pub struct UpdateArticleInput {
    pub uri: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
}

pub async fn update_article(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UpdateArticleInput>,
) -> ApiResult<Json<Article>> {
    let mut errors = Vec::new();
    if let Some(ref title) = input.title {
        if let Err(e) = fx_core::validation::validate_title(title) {
            errors.push(e);
        }
    }
    if let Some(ref content) = input.content {
        if let Err(e) = fx_core::validation::validate_article_content(content) {
            errors.push(e);
        }
    }
    if !errors.is_empty() {
        return Err(AppError(fx_core::Error::Validation(errors)));
    }

    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;

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
            let rendered = render_content(&format, content, &repo_path)?;
            let _ = tokio::fs::write(repo_path.join("content.html"), &rendered).await;
        }

        let hash = content_hash(content);
        article_service::update_article_content_hash(&state.pool, &input.uri, &hash).await?;

        if let Err(e) = state.pijul.record(&node_id, "Update article") {
            tracing::warn!("pijul record failed for {node_id}: {e}");
        }
    }

    let article = article_service::get_article_any_visibility(&state.pool, &input.uri).await?;
    Ok(Json(article))
}

// --- Delete article ---

#[derive(serde::Deserialize)]
pub struct DeleteArticleInput {
    pub uri: String,
}

pub async fn delete_article(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<DeleteArticleInput>,
) -> ApiResult<StatusCode> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;

    article_service::delete_article(&state.pool, &input.uri).await?;

    let node_id = uri_to_node_id(&input.uri);
    let repo_path = state.pijul.repo_path(&node_id);
    let _ = tokio::fs::remove_dir_all(&repo_path).await;

    Ok(StatusCode::NO_CONTENT)
}

// --- Access control (paywall) ---

#[derive(serde::Deserialize)]
pub struct SetRestrictedInput {
    pub uri: String,
    pub restricted: bool,
}

pub async fn set_restricted(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<SetRestrictedInput>,
) -> ApiResult<StatusCode> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;
    article_service::set_restricted(&state.pool, &input.uri, input.restricted).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct AccessGrantInput {
    pub uri: String,
    pub grantee_did: String,
}

pub async fn grant_access(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<AccessGrantInput>,
) -> ApiResult<StatusCode> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;
    article_service::grant_access(&state.pool, &input.uri, &input.grantee_did).await?;
    Ok(StatusCode::OK)
}

pub async fn revoke_access(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<AccessGrantInput>,
) -> ApiResult<StatusCode> {
    let owner = article_service::get_article_owner(&state.pool, &input.uri).await?;
    require_owner(Some(&owner), &user.did)?;
    article_service::revoke_access(&state.pool, &input.uri, &input.grantee_did).await?;
    Ok(StatusCode::OK)
}

pub async fn list_access_grants(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<article_service::AccessGrant>>> {
    let owner = article_service::get_article_owner(&state.pool, &uri).await?;
    require_owner(Some(&owner), &user.did)?;
    let grants = article_service::list_access_grants(&state.pool, &uri).await?;
    Ok(Json(grants))
}

// --- Search ---

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<i64>,
}

pub async fn search_articles(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let engine = fx_search::SearchEngine::new(state.pool.clone());
    let uris = engine.search(&q.q, limit).await
        .map_err(|e| AppError(fx_core::Error::Internal(e.to_string())))?;

    let articles = article_service::get_articles_by_uris(&state.pool, state.instance_mode, &uris).await?;
    Ok(Json(articles))
}
