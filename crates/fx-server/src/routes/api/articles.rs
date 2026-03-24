use axum::{
    Json,
    extract::{Multipart, Query, State},
    http::{HeaderMap, StatusCode},
};
use fx_core::models::*;

use crate::error::{ApiError, ApiResult, require_did, require_owner};
use crate::state::AppState;
use super::{AuthDid, RequireAuth, TagIdQuery, UriQuery, DidQuery, ARTICLE_SELECT, content_hash, tid, uri_to_node_id, session_from_headers, chrono_now};

pub async fn list_articles(State(state): State<AppState>) -> ApiResult<Json<Vec<Article>>> {
    let articles =
        sqlx::query_as::<_, Article>(&format!("{ARTICLE_SELECT} ORDER BY a.created_at DESC LIMIT 50"))
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(articles))
}

pub async fn get_article(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Article>> {
    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_SELECT} WHERE a.at_uri = ?"))
        .bind(&uri)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound("article not found".into()))?;
    Ok(Json(article))
}

pub async fn get_article_content(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<ArticleContent>> {
    // Look up content_format from DB
    let format: String = sqlx::query_scalar("SELECT content_format FROM articles WHERE at_uri = ?")
        .bind(&uri)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound("article not found".into()))?;

    let node_id = uri_to_node_id(&uri);
    let repo = state.pijul.repo_path(&node_id);

    let src_ext = if format == "markdown" { "md" } else { "typ" };
    let src_path = repo.join(format!("content.{src_ext}"));
    let html_path = repo.join("content.html");

    // Fallback: also try content.typ for old articles stored before markdown support
    let source = std::fs::read_to_string(&src_path)
        .or_else(|_| std::fs::read_to_string(repo.join("content.typ")))
        .map_err(|_| ApiError::NotFound("content not found".into()))?;

    // Use cached HTML if it exists and is newer than the source
    let html = if html_path.exists() && is_newer(&html_path, &src_path) {
        std::fs::read_to_string(&html_path)?
    } else {
        let rendered = match format.as_str() {
            "markdown" => fx_render::render_markdown_to_html(&source)
                .map_err(|e| ApiError::Internal(e.to_string()))?,
            _ => fx_render::render_typst_to_html_with_images(&source, &repo)
                .map_err(|e| ApiError::Internal(e.to_string()))?,
        };
        let _ = std::fs::write(&html_path, &rendered);
        rendered
    };

    Ok(Json(ArticleContent { source, html }))
}

fn is_newer(a: &std::path::Path, b: &std::path::Path) -> bool {
    let Ok(a_meta) = a.metadata() else { return false };
    let Ok(b_meta) = b.metadata() else { return true };
    let Ok(a_mod) = a_meta.modified() else { return false };
    let Ok(b_mod) = b_meta.modified() else { return true };
    a_mod >= b_mod
}

pub async fn get_article_prereqs(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<ArticlePrereqRow>>> {
    let prereqs = sqlx::query_as::<_, ArticlePrereqRow>(
        "SELECT ap.tag_id, ap.prereq_type, t.name as tag_name
         FROM article_prereqs ap
         JOIN tags t ON t.id = ap.tag_id
         WHERE ap.article_uri = ?",
    )
    .bind(&uri)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(prereqs))
}

pub async fn get_article_forks(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<ForkWithTitle>>> {
    let forks = sqlx::query_as::<_, ForkWithTitle>(
        "SELECT f.fork_uri, f.forked_uri, f.vote_score, a.title, a.did, p.handle AS author_handle
         FROM forks f
         JOIN articles a ON a.at_uri = f.forked_uri
         LEFT JOIN profiles p ON a.did = p.did
         WHERE f.source_uri = ?
         ORDER BY f.vote_score DESC",
    )
    .bind(&uri)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(forks))
}

pub async fn create_article(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    headers: HeaderMap,
    Json(input): Json<CreateArticle>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    let at_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::ARTICLE, tid());

    let node_id = uri_to_node_id(&at_uri);
    state
        .pijul
        .init_repo(&node_id)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let repo_path = state.pijul.repo_path(&node_id);
    let src_ext = if input.content_format == "markdown" { "md" } else { "typ" };
    std::fs::write(repo_path.join(format!("content.{src_ext}")), &input.content)?;

    let rendered_html = match input.content_format.as_str() {
        "markdown" => fx_render::render_markdown_to_html(&input.content)
            .map_err(|e| {
                tracing::warn!("render error: {e}");
                ApiError::BadRequest(e.to_string())
            })?,
        _ => fx_render::render_typst_to_html_with_images(&input.content, &repo_path)
            .map_err(|e| {
                tracing::warn!("render error: {e}");
                ApiError::BadRequest(e.to_string())
            })?,
    };
    // Cache rendered HTML
    let _ = std::fs::write(repo_path.join("content.html"), &rendered_html);

    // Record initial change in pijul
    if let Err(e) = state.pijul.record(&node_id, "Initial publish") {
        tracing::warn!("pijul record failed for {node_id}: {e}");
    }

    let hash = content_hash(&input.content);
    let lang = input.lang.as_deref().unwrap_or("zh");

    // Resolve translation_group: if translating an existing article, use its group
    let translation_group = if let Some(ref source_uri) = input.translation_of {
        let group: Option<String> = sqlx::query_scalar(
            "SELECT COALESCE(translation_group, at_uri) FROM articles WHERE at_uri = ?"
        )
        .bind(source_uri)
        .fetch_optional(&state.pool)
        .await?;
        let g = group.unwrap_or_else(|| source_uri.clone());
        // Also update source article's translation_group if null
        sqlx::query(
            "UPDATE articles SET translation_group = ? WHERE at_uri = ? AND translation_group IS NULL"
        )
        .bind(&g)
        .bind(source_uri)
        .execute(&state.pool)
        .await?;
        Some(g)
    } else {
        None
    };

    let license = input.license.as_deref().unwrap_or("CC-BY-NC-SA-4.0");

    // Use a transaction for the multi-step article creation
    let mut tx = state.pool.begin().await?;

    sqlx::query(
        "INSERT INTO articles (at_uri, did, title, description, content_hash, content_format, lang, translation_group, license, prereq_threshold)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0.8)",
    )
    .bind(&at_uri)
    .bind(&did)
    .bind(&input.title)
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(&hash)
    .bind(&input.content_format)
    .bind(lang)
    .bind(&translation_group)
    .bind(license)
    .execute(&mut *tx)
    .await?;

    for tag_id in &input.tags {
        // Auto-create tag if it doesn't exist
        sqlx::query(
            "INSERT OR IGNORE INTO tags (id, name, created_by) VALUES (?, ?, ?)"
        )
            .bind(tag_id)
            .bind(tag_id)
            .bind(&did)
            .execute(&mut *tx)
            .await?;

        sqlx::query("INSERT OR IGNORE INTO article_tags (article_uri, tag_id) VALUES (?, ?)")
            .bind(&at_uri)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
    }

    for prereq in &input.prereqs {
        sqlx::query(
            "INSERT OR IGNORE INTO article_prereqs (article_uri, tag_id, prereq_type) VALUES (?, ?, ?)",
        )
        .bind(&at_uri)
        .bind(&prereq.tag_id)
        .bind(prereq.prereq_type.as_str())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    if let Some((_did, pds_url, access_jwt)) = session_from_headers(&state.pool, &headers).await {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::ARTICLE,
            "title": input.title,
            "description": input.description.as_deref().unwrap_or(""),
            "contentFormat": input.content_format,
            "tags": input.tags,
            "createdAt": chrono_now(),
        });
        let _ = state.at_client.create_record(
            &pds_url,
            &access_jwt,
            &fx_atproto::client::CreateRecordInput {
                repo: _did,
                collection: fx_atproto::lexicon::ARTICLE.to_string(),
                record,
                rkey: None,
            },
        ).await;
    }

    if did != "did:plc:anonymous" {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO user_bookmarks (did, article_uri, folder_path) VALUES (?, ?, '我的文章')"
        )
        .bind(&did)
        .bind(&at_uri)
        .execute(&state.pool)
        .await;
    }

    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_SELECT} WHERE a.at_uri = ?"))
        .bind(&at_uri)
        .fetch_one(&state.pool)
        .await?;

    Ok((StatusCode::CREATED, Json(article)))
}

// --- Bulk article metadata ---

#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct ArticleTagRow {
    article_uri: String,
    tag_id: String,
    tag_name: String,
}

pub async fn get_all_article_tags(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ArticleTagRow>>> {
    let rows = sqlx::query_as::<_, ArticleTagRow>(
        "SELECT at2.article_uri, at2.tag_id, t.name as tag_name
         FROM article_tags at2
         JOIN tags t ON t.id = at2.tag_id
         ORDER BY at2.article_uri",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct ArticlePrereqBulkRow {
    article_uri: String,
    tag_id: String,
    prereq_type: String,
    tag_name: String,
}

pub async fn get_all_article_prereqs(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ArticlePrereqBulkRow>>> {
    let rows = sqlx::query_as::<_, ArticlePrereqBulkRow>(
        "SELECT ap.article_uri, ap.tag_id, ap.prereq_type, t.name as tag_name
         FROM article_prereqs ap
         JOIN tags t ON t.id = ap.tag_id
         ORDER BY ap.article_uri",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn get_articles_by_tag(
    State(state): State<AppState>,
    Query(TagIdQuery { tag_id }): Query<TagIdQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    // Collect the tag itself plus all descendant tags from skill_tree_edges
    let descendant_tags: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE descendants(tag) AS (
           SELECT ?
           UNION
           SELECT e.child_tag FROM skill_tree_edges e JOIN descendants d ON e.parent_tag = d.tag
         )
         SELECT tag FROM descendants",
    )
    .bind(&tag_id)
    .fetch_all(&state.pool)
    .await?;

    if descendant_tags.is_empty() {
        return Ok(Json(vec![]));
    }

    // Build placeholders for IN clause
    let placeholders: Vec<&str> = descendant_tags.iter().map(|_| "?").collect();
    let in_clause = placeholders.join(",");
    let sql = format!(
        "{ARTICLE_SELECT} JOIN article_tags at2 ON at2.article_uri = a.at_uri \
         WHERE at2.tag_id IN ({in_clause}) \
         GROUP BY a.at_uri \
         ORDER BY a.created_at DESC"
    );

    let mut query = sqlx::query_as::<_, Article>(&sql);
    for t in &descendant_tags {
        query = query.bind(t);
    }
    let articles = query.fetch_all(&state.pool).await?;
    Ok(Json(articles))
}

pub async fn get_articles_by_did(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let articles = sqlx::query_as::<_, Article>(
        &format!("{ARTICLE_SELECT} WHERE a.did = ? ORDER BY a.created_at DESC"),
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(articles))
}

// --- Translations ---

pub async fn get_translations(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    // Find the translation group for this article
    let group: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(translation_group, at_uri) FROM articles WHERE at_uri = ?"
    )
    .bind(&uri)
    .fetch_optional(&state.pool)
    .await?;

    let Some(group) = group else {
        return Ok(Json(vec![]));
    };

    let articles = sqlx::query_as::<_, Article>(
        &format!("{ARTICLE_SELECT} WHERE a.translation_group = ? AND a.at_uri != ? ORDER BY a.lang"),
    )
    .bind(&group)
    .bind(&uri)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(articles))
}

// --- Fork ---

pub async fn fork_article(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    headers: HeaderMap,
    Json(input): Json<ForkArticleInput>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    let source = sqlx::query_as::<_, Article>(&format!("{ARTICLE_SELECT} WHERE a.at_uri = ?"))
        .bind(&input.uri)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound("article not found".into()))?;

    let fork_at_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::ARTICLE, tid());
    let fork_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::FORK, tid());

    let source_node_id = uri_to_node_id(&input.uri);
    let fork_node_id = uri_to_node_id(&fork_at_uri);
    state
        .pijul
        .fork(&source_node_id, &fork_node_id)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    sqlx::query(
        "INSERT INTO articles (at_uri, did, title, content_hash, content_format, prereq_threshold)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&fork_at_uri)
    .bind(&did)
    .bind(&format!("Fork: {}", source.title))
    .bind(&source.content_hash)
    .bind(&source.content_format)
    .bind(source.prereq_threshold)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        "INSERT OR IGNORE INTO article_prereqs (article_uri, tag_id, prereq_type)
         SELECT ?, tag_id, prereq_type FROM article_prereqs WHERE article_uri = ?",
    )
    .bind(&fork_at_uri)
    .bind(&input.uri)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        "INSERT OR IGNORE INTO article_tags (article_uri, tag_id)
         SELECT ?, tag_id FROM article_tags WHERE article_uri = ?",
    )
    .bind(&fork_at_uri)
    .bind(&input.uri)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        "INSERT INTO forks (fork_uri, source_uri, forked_uri) VALUES (?, ?, ?)",
    )
    .bind(&fork_uri)
    .bind(&input.uri)
    .bind(&fork_at_uri)
    .execute(&state.pool)
    .await?;

    if let Some((_did, pds_url, access_jwt)) = session_from_headers(&state.pool, &headers).await {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::FORK,
            "source": input.uri,
            "fork": fork_at_uri,
            "createdAt": chrono_now(),
        });
        let _ = state.at_client.create_record(
            &pds_url,
            &access_jwt,
            &fx_atproto::client::CreateRecordInput {
                repo: _did,
                collection: fx_atproto::lexicon::FORK.to_string(),
                record,
                rkey: None,
            },
        ).await;
    }

    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_SELECT} WHERE a.at_uri = ?"))
        .bind(&fork_at_uri)
        .fetch_one(&state.pool)
        .await?;

    Ok((StatusCode::CREATED, Json(article)))
}

#[derive(serde::Deserialize)]
pub struct ForkArticleInput {
    uri: String,
}

// --- Image upload ---

const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
const ALLOWED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "svg", "webp"];

pub async fn upload_image(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    mut multipart: Multipart,
) -> ApiResult<Json<ImageUploadResponse>> {
    require_did(&did)?;

    let mut article_uri: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::BadRequest(format!("Multipart error: {e}"))
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "article_uri" => {
                article_uri = Some(field.text().await.map_err(|e| {
                    ApiError::BadRequest(e.to_string())
                })?);
            }
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                file_data = Some(field.bytes().await.map_err(|e| {
                    ApiError::BadRequest(e.to_string())
                })?.to_vec());
            }
            _ => {}
        }
    }

    let uri = article_uri.ok_or(ApiError::BadRequest("Missing article_uri".into()))?;
    let original_name = file_name.ok_or(ApiError::BadRequest("Missing file".into()))?;
    let data = file_data.ok_or(ApiError::BadRequest("Missing file data".into()))?;

    if data.len() > MAX_IMAGE_SIZE {
        return Err(ApiError::BadRequest("File too large (max 10MB)".into()));
    }

    // Verify article belongs to this user
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM articles WHERE at_uri = ?")
        .bind(&uri)
        .fetch_optional(&state.pool)
        .await?;
    require_owner(owner.as_deref(), &did)?;

    // Validate extension
    let ext = std::path::Path::new(&original_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(ApiError::BadRequest(format!("Unsupported file type: {ext}")));
    }

    // Sanitize filename: keep alphanumeric, dash, underscore, dot
    let safe_name: String = original_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect();

    let node_id = uri_to_node_id(&uri);
    let repo_path = state.pijul.repo_path(&node_id);
    let dest = repo_path.join(&safe_name);

    std::fs::write(&dest, &data)?;

    // Invalidate HTML cache since images may affect rendering
    let _ = std::fs::remove_file(repo_path.join("content.html"));

    // Record image addition in pijul
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
    RequireAuth(did): RequireAuth,
    Json(input): Json<UpdateArticleInput>,
) -> ApiResult<Json<Article>> {
    // Verify ownership
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM articles WHERE at_uri = ?")
        .bind(&input.uri)
        .fetch_optional(&state.pool)
        .await?;
    require_owner(owner.as_deref(), &did)?;

    if let Some(ref title) = input.title {
        sqlx::query("UPDATE articles SET title = ?, updated_at = datetime('now') WHERE at_uri = ?")
            .bind(title)
            .bind(&input.uri)
            .execute(&state.pool)
            .await?;
    }
    if let Some(ref desc) = input.description {
        sqlx::query("UPDATE articles SET description = ?, updated_at = datetime('now') WHERE at_uri = ?")
            .bind(desc)
            .bind(&input.uri)
            .execute(&state.pool)
            .await?;
    }
    if let Some(ref content) = input.content {
        let format: String = sqlx::query_scalar("SELECT content_format FROM articles WHERE at_uri = ?")
            .bind(&input.uri)
            .fetch_one(&state.pool)
            .await?;

        let node_id = uri_to_node_id(&input.uri);
        let repo_path = state.pijul.repo_path(&node_id);
        let src_ext = if format == "markdown" { "md" } else { "typ" };
        std::fs::write(repo_path.join(format!("content.{src_ext}")), content)?;

        let rendered = match format.as_str() {
            "markdown" => fx_render::render_markdown_to_html(content)
                .map_err(|e| {
                    tracing::warn!("render error: {e}");
                    ApiError::BadRequest(e.to_string())
                })?,
            _ => fx_render::render_typst_to_html_with_images(content, &repo_path)
                .map_err(|e| {
                    tracing::warn!("render error: {e}");
                    ApiError::BadRequest(e.to_string())
                })?,
        };
        let _ = std::fs::write(repo_path.join("content.html"), &rendered);

        let hash = content_hash(content);
        sqlx::query("UPDATE articles SET content_hash = ?, updated_at = datetime('now') WHERE at_uri = ?")
            .bind(&hash)
            .bind(&input.uri)
            .execute(&state.pool)
            .await?;

        if let Err(e) = state.pijul.record(&node_id, "Update article") {
            tracing::warn!("pijul record failed for {node_id}: {e}");
        }
    }

    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_SELECT} WHERE a.at_uri = ?"))
        .bind(&input.uri)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(article))
}

// --- Delete article ---

#[derive(serde::Deserialize)]
pub struct DeleteArticleInput {
    pub uri: String,
}

pub async fn delete_article(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<DeleteArticleInput>,
) -> ApiResult<StatusCode> {
    // Verify ownership
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM articles WHERE at_uri = ?")
        .bind(&input.uri)
        .fetch_optional(&state.pool)
        .await?;
    require_owner(owner.as_deref(), &did)?;

    // Delete associated data
    let mut tx = state.pool.begin().await?;

    sqlx::query("DELETE FROM comments WHERE article_uri = ?")
        .bind(&input.uri).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM article_tags WHERE article_uri = ?")
        .bind(&input.uri).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM article_prereqs WHERE article_uri = ?")
        .bind(&input.uri).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM forks WHERE source_uri = ? OR forked_uri = ?")
        .bind(&input.uri).bind(&input.uri).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM user_bookmarks WHERE article_uri = ?")
        .bind(&input.uri).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM votes WHERE target_uri = ?")
        .bind(&input.uri).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM series_articles WHERE article_uri = ?")
        .bind(&input.uri).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM articles WHERE at_uri = ?")
        .bind(&input.uri).execute(&mut *tx).await?;

    tx.commit().await?;

    // Clean up repo files
    let node_id = uri_to_node_id(&input.uri);
    let repo_path = state.pijul.repo_path(&node_id);
    let _ = std::fs::remove_dir_all(&repo_path);

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_image(
    State(state): State<AppState>,
    Query(q): Query<ImageQuery>,
) -> ApiResult<(HeaderMap, Vec<u8>)> {
    let node_id = uri_to_node_id(&q.uri);
    let repo_path = state.pijul.repo_path(&node_id);

    // Sanitize name to prevent directory traversal
    let name = std::path::Path::new(&q.name)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(ApiError::BadRequest("invalid file name".into()))?;

    let path = repo_path.join(name);
    if !path.exists() {
        return Err(ApiError::NotFound("image not found".into()));
    }

    let data = std::fs::read(&path)?;

    let content_type = match std::path::Path::new(name).extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    };

    let mut headers = HeaderMap::new();
    headers.insert("content-type", content_type.parse().unwrap());
    headers.insert("cache-control", "public, max-age=86400".parse().unwrap());
    Ok((headers, data))
}
