use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use fx_core::models::*;

use crate::error::{ApiError, ApiResult, require_owner};
use crate::state::AppState;
use super::{RequireAuth, tid, content_hash, uri_to_node_id, session_from_headers, chrono_now, ARTICLE_SELECT};

// --- List drafts ---

pub async fn list_drafts(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
) -> ApiResult<Json<Vec<Draft>>> {
    let drafts = sqlx::query_as::<_, Draft>(
        "SELECT * FROM drafts WHERE did = ? ORDER BY updated_at DESC",
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(drafts))
}

// --- Save draft ---

pub async fn save_draft(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    headers: HeaderMap,
    Json(input): Json<SaveDraft>,
) -> ApiResult<(StatusCode, Json<Draft>)> {
    let id = tid();
    let tags_json = serde_json::to_string(&input.tags).unwrap_or_else(|_| "[]".into());
    let prereqs_json = serde_json::to_string(&input.prereqs).unwrap_or_else(|_| "[]".into());
    let lang = input.lang.as_deref().unwrap_or("zh");
    let license = input.license.as_deref().unwrap_or("CC-BY-NC-SA-4.0");

    sqlx::query(
        "INSERT INTO drafts (id, did, title, description, content, content_format, lang, license, tags, prereqs)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&did)
    .bind(&input.title)
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(&input.content)
    .bind(&input.content_format)
    .bind(lang)
    .bind(license)
    .bind(&tags_json)
    .bind(&prereqs_json)
    .execute(&state.pool)
    .await?;

    // Sync to PDS
    if let Some((_did, pds_url, access_jwt)) = session_from_headers(&state.pool, &headers).await {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::DRAFT,
            "title": input.title,
            "description": input.description.as_deref().unwrap_or(""),
            "contentFormat": input.content_format,
            "tags": input.tags,
            "createdAt": chrono_now(),
        });
        if let Ok(output) = state.at_client.create_record(
            &pds_url,
            &access_jwt,
            &fx_atproto::client::CreateRecordInput {
                repo: _did,
                collection: fx_atproto::lexicon::DRAFT.to_string(),
                record,
                rkey: Some(id.clone()),
            },
        ).await {
            let _ = sqlx::query("UPDATE drafts SET at_uri = ? WHERE id = ?")
                .bind(&output.uri)
                .bind(&id)
                .execute(&state.pool)
                .await;
        }
    }

    let draft = sqlx::query_as::<_, Draft>("SELECT * FROM drafts WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;

    Ok((StatusCode::CREATED, Json(draft)))
}

// --- Update draft ---

pub async fn update_draft(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<UpdateDraft>,
) -> ApiResult<Json<Draft>> {
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM drafts WHERE id = ?")
        .bind(&input.id)
        .fetch_optional(&state.pool)
        .await?;
    require_owner(owner.as_deref(), &did)?;

    if let Some(ref title) = input.title {
        sqlx::query("UPDATE drafts SET title = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(title).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref desc) = input.description {
        sqlx::query("UPDATE drafts SET description = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(desc).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref content) = input.content {
        sqlx::query("UPDATE drafts SET content = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(content).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref fmt) = input.content_format {
        sqlx::query("UPDATE drafts SET content_format = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(fmt).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref lang) = input.lang {
        sqlx::query("UPDATE drafts SET lang = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(lang).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref license) = input.license {
        sqlx::query("UPDATE drafts SET license = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(license).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref tags) = input.tags {
        let json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".into());
        sqlx::query("UPDATE drafts SET tags = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(&json).bind(&input.id).execute(&state.pool).await?;
    }
    if let Some(ref prereqs) = input.prereqs {
        let json = serde_json::to_string(prereqs).unwrap_or_else(|_| "[]".into());
        sqlx::query("UPDATE drafts SET prereqs = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(&json).bind(&input.id).execute(&state.pool).await?;
    }

    let draft = sqlx::query_as::<_, Draft>("SELECT * FROM drafts WHERE id = ?")
        .bind(&input.id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(draft))
}

// --- Delete draft ---

#[derive(serde::Deserialize)]
pub struct DeleteDraftInput {
    pub id: String,
}

pub async fn delete_draft(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    headers: HeaderMap,
    Json(input): Json<DeleteDraftInput>,
) -> ApiResult<StatusCode> {
    let draft = sqlx::query_as::<_, Draft>("SELECT * FROM drafts WHERE id = ?")
        .bind(&input.id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound("draft not found".into()))?;
    require_owner(Some(draft.did.as_str()), &did)?;

    // Delete from PDS
    if let Some(ref _at_uri) = draft.at_uri {
        if let Some((_did, pds_url, access_jwt)) = session_from_headers(&state.pool, &headers).await {
            let _ = state.at_client.delete_record(
                &pds_url,
                &access_jwt,
                &fx_atproto::client::DeleteRecordInput {
                    repo: _did,
                    collection: fx_atproto::lexicon::DRAFT.to_string(),
                    rkey: input.id.clone(),
                },
            ).await;
        }
    }

    sqlx::query("DELETE FROM drafts WHERE id = ?")
        .bind(&input.id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Publish draft (convert to article) ---

#[derive(serde::Deserialize)]
pub struct PublishDraftInput {
    pub id: String,
}

pub async fn publish_draft(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    headers: HeaderMap,
    Json(input): Json<PublishDraftInput>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    let draft = sqlx::query_as::<_, Draft>("SELECT * FROM drafts WHERE id = ?")
        .bind(&input.id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound("draft not found".into()))?;
    require_owner(Some(draft.did.as_str()), &did)?;

    let tags: Vec<String> = serde_json::from_str(&draft.tags).unwrap_or_default();
    let prereqs: Vec<ArticlePrereq> = serde_json::from_str(&draft.prereqs).unwrap_or_default();

    // Create article (same flow as create_article)
    let at_uri = format!("at://{}/{}/{}", did, fx_atproto::lexicon::ARTICLE, tid());
    let node_id = uri_to_node_id(&at_uri);
    state.pijul.init_repo(&node_id)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let repo_path = state.pijul.repo_path(&node_id);
    let src_ext = if draft.content_format == "markdown" { "md" } else { "typ" };
    std::fs::write(repo_path.join(format!("content.{src_ext}")), &draft.content)?;

    let rendered_html = match draft.content_format.as_str() {
        "markdown" => fx_render::render_markdown_to_html(&draft.content)
            .map_err(|e| ApiError::BadRequest(e.to_string()))?,
        _ => fx_render::render_typst_to_html_with_images(&draft.content, &repo_path)
            .map_err(|e| ApiError::BadRequest(e.to_string()))?,
    };
    let _ = std::fs::write(repo_path.join("content.html"), &rendered_html);

    if let Err(e) = state.pijul.record(&node_id, "Initial publish") {
        tracing::warn!("pijul record failed for {node_id}: {e}");
    }

    let hash = content_hash(&draft.content);

    let mut tx = state.pool.begin().await?;

    sqlx::query(
        "INSERT INTO articles (at_uri, did, title, description, content_hash, content_format, lang, license, prereq_threshold)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0.8)",
    )
    .bind(&at_uri)
    .bind(&did)
    .bind(&draft.title)
    .bind(&draft.description)
    .bind(&hash)
    .bind(&draft.content_format)
    .bind(&draft.lang)
    .bind(&draft.license)
    .execute(&mut *tx)
    .await?;

    for tag_id in &tags {
        sqlx::query("INSERT OR IGNORE INTO tags (id, name, created_by) VALUES (?, ?, ?)")
            .bind(tag_id).bind(tag_id).bind(&did)
            .execute(&mut *tx).await?;
        sqlx::query("INSERT OR IGNORE INTO article_tags (article_uri, tag_id) VALUES (?, ?)")
            .bind(&at_uri).bind(tag_id)
            .execute(&mut *tx).await?;
    }

    for prereq in &prereqs {
        sqlx::query(
            "INSERT OR IGNORE INTO article_prereqs (article_uri, tag_id, prereq_type) VALUES (?, ?, ?)",
        )
        .bind(&at_uri)
        .bind(&prereq.tag_id)
        .bind(prereq.prereq_type.as_str())
        .execute(&mut *tx)
        .await?;
    }

    // Delete draft from DB
    sqlx::query("DELETE FROM drafts WHERE id = ?")
        .bind(&input.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // PDS: create article record, delete draft record
    if let Some((_did, pds_url, access_jwt)) = session_from_headers(&state.pool, &headers).await {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::ARTICLE,
            "title": draft.title,
            "description": draft.description,
            "contentFormat": draft.content_format,
            "tags": tags,
            "createdAt": chrono_now(),
        });
        let _ = state.at_client.create_record(
            &pds_url,
            &access_jwt,
            &fx_atproto::client::CreateRecordInput {
                repo: _did.clone(),
                collection: fx_atproto::lexicon::ARTICLE.to_string(),
                record,
                rkey: None,
            },
        ).await;

        // Delete draft from PDS
        if draft.at_uri.is_some() {
            let _ = state.at_client.delete_record(
                &pds_url,
                &access_jwt,
                &fx_atproto::client::DeleteRecordInput {
                    repo: _did,
                    collection: fx_atproto::lexicon::DRAFT.to_string(),
                    rkey: input.id.clone(),
                },
            ).await;
        }
    }

    // Auto-bookmark
    let _ = sqlx::query(
        "INSERT OR IGNORE INTO user_bookmarks (did, article_uri, folder_path) VALUES (?, ?, '我的文章')"
    )
    .bind(&did)
    .bind(&at_uri)
    .execute(&state.pool)
    .await;

    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_SELECT} WHERE a.at_uri = ?"))
        .bind(&at_uri)
        .fetch_one(&state.pool)
        .await?;

    Ok((StatusCode::CREATED, Json(article)))
}
