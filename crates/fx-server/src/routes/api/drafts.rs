use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use fx_core::content::ContentFormat;
use fx_core::models::*;
use fx_core::region::default_visibility;
use fx_core::services::{article_service, draft_service, version_service};
use fx_core::validation;

use crate::error::{AppError, ApiResult, require_owner};
use crate::state::AppState;
use crate::auth::{Auth, WriteAuth, pds_create_record, pds_delete_record};
use fx_core::util::{tid, content_hash, uri_to_node_id, now_rfc3339};

pub async fn list_drafts(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<Draft>>> {
    let drafts = draft_service::list_drafts(&state.pool, &user.did).await?;
    Ok(Json(drafts))
}

pub async fn save_draft(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<SaveDraft>,
) -> ApiResult<(StatusCode, Json<Draft>)> {
    validation::validate_save_draft(&input)?;

    let id = tid();

    let _draft = draft_service::save_draft(&state.pool, &id, &user.did, &input).await?;

    // Sync to PDS
    let record = serde_json::json!({
        "$type": fx_atproto::lexicon::DRAFT,
        "title": input.title,
        "description": input.description.as_deref().unwrap_or(""),
        "contentFormat": input.content_format,
        "tags": input.tags,
        "createdAt": now_rfc3339(),
    });
    if let Some(uri) = pds_create_record(&state, &user.token, fx_atproto::lexicon::DRAFT, record, Some(id.clone()), "create draft").await {
        let _ = draft_service::set_draft_at_uri(&state.pool, &id, &uri).await;
    }

    // Re-fetch to get potential at_uri update
    let draft = draft_service::get_draft(&state.pool, &id).await?;
    Ok((StatusCode::CREATED, Json(draft)))
}

pub async fn update_draft(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
    Json(mut input): Json<UpdateDraft>,
) -> ApiResult<Json<Draft>> {
    input.id = id;
    let draft = draft_service::update_draft(&state.pool, &user.did, &input).await?;
    Ok(Json(draft))
}

pub async fn delete_draft(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
) -> ApiResult<StatusCode> {
    let draft = draft_service::get_draft(&state.pool, &id).await?;
    require_owner(Some(draft.did.as_str()), &user.did)?;

    // Delete from PDS
    if draft.at_uri.is_some() {
        pds_delete_record(&state, &user.token, fx_atproto::lexicon::DRAFT, id.clone(), "delete draft").await;
    }

    draft_service::delete_draft(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Publish draft (convert to article) ---

pub async fn publish_draft(
    State(state): State<AppState>,
    Path(id): Path<String>,
    WriteAuth(user): WriteAuth,
) -> ApiResult<(StatusCode, Json<Article>)> {
    let draft = draft_service::get_draft(&state.pool, &id).await?;
    require_owner(Some(draft.did.as_str()), &user.did)?;

    let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::ARTICLE, tid());

    // Set up pijul repo and write content
    let node_id = uri_to_node_id(&at_uri);
    state.pijul.init_repo(&node_id)
        .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;

    let repo_path = state.pijul.repo_path(&node_id);
    let src_ext = if draft.content_format == ContentFormat::Markdown { "md" } else { "typ" };
    tokio::fs::write(repo_path.join(format!("content.{src_ext}")), &draft.content).await?;

    let rendered_html = match draft.content_format.as_str() {
        "markdown" => fx_render::render_markdown_to_html(&draft.content)
            .map_err(|e| AppError(fx_core::Error::Render(e.to_string())))?,
        _ => fx_render::render_typst_to_html_with_images(&draft.content, &repo_path)
            .map_err(|e| AppError(fx_core::Error::Render(e.to_string())))?,
    };
    let _ = tokio::fs::write(repo_path.join("content.html"), &rendered_html).await;

    match state.pijul.record(&node_id, "Initial publish", Some(&user.did)) {
        Ok(Some((hash, new_state))) => {
            let _ = version_service::record_version(
                &state.pool, &at_uri, &hash, &user.did, "Initial publish", &draft.content,
            ).await;
            super::articles::publish_pijul_ref_update(&state, &user.token, &at_uri, &user.did, &hash, &new_state).await;
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("pijul record failed for {node_id}: {e}"),
    }

    let hash = content_hash(&draft.content);

    // Publish: create article + tags + prereqs + delete draft in one transaction
    let (tags, _prereqs) = draft_service::publish_to_article(
        &state.pool, &draft, &at_uri, &hash, default_visibility(user.phone_verified),
    ).await?;

    // PDS: create article record, delete draft record
    let record = serde_json::json!({
        "$type": fx_atproto::lexicon::ARTICLE,
        "title": draft.title,
        "description": draft.description,
        "contentFormat": draft.content_format,
        "tags": tags,
        "createdAt": now_rfc3339(),
    });
    pds_create_record(&state, &user.token, fx_atproto::lexicon::ARTICLE, record, None, "publish article").await;
    if draft.at_uri.is_some() {
        pds_delete_record(&state, &user.token, fx_atproto::lexicon::DRAFT, id.clone(), "delete published draft").await;
    }

    let article = article_service::get_article_any_visibility(&state.pool, &at_uri).await?;
    Ok((StatusCode::CREATED, Json(article)))
}
