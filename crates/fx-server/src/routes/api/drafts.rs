use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use fx_core::models::*;
use fx_core::region::default_visibility;
use fx_core::services::{article_service, draft_service};
use fx_core::validation;

use crate::error::{AppError, ApiResult, require_owner};
use crate::state::AppState;
use crate::auth::{Auth, WriteAuth, pds_create_record, pds_delete_record};
use fx_core::util::{tid, content_hash, now_rfc3339};

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
        "description": input.summary.as_deref().unwrap_or(""),
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

    let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::WORK, tid());

    // Publish the draft content as a PDS-blob-backed article.
    let publish = super::articles::publish_article_blob(
        &state, &at_uri, &user.did, &user.token, &draft.content, draft.content_format,
        super::articles::SummaryInput { user_source: Some(draft.summary.as_str()) },
    ).await?;

    let hash = content_hash(&draft.content);

    // Publish: create article + tags + prereqs + delete draft in one transaction
    let (tags, _prereqs) = draft_service::publish_to_article(
        &state.pool, &draft, &at_uri, &hash, default_visibility(user.phone_verified),
    ).await?;

    // Persist the blob manifest on the localization row.
    if let Some(ref manifest) = publish.blob_manifest {
        let _ = sqlx::query("UPDATE article_localizations SET content_manifest = $1 WHERE at_uri = $2")
            .bind(manifest).bind(&at_uri).execute(&state.pool).await;
    }

    // PDS: create article record (new lexicon shape) and delete the
    // draft record it replaces.
    if let Some(ref manifest) = publish.blob_manifest {
        let files: Vec<serde_json::Value> = manifest
            .get("files")
            .and_then(|f| f.as_array())
            .map(|arr| arr.iter().map(|f| {
                let cid = f.get("cid").and_then(|c| c.as_str()).unwrap_or_default();
                let mime = f.get("mime").and_then(|m| m.as_str()).unwrap_or("text/plain");
                let size = f.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                serde_json::json!({
                    "path": f.get("path").cloned().unwrap_or(serde_json::Value::Null),
                    "blob": {
                        "$type": "blob",
                        "ref": { "$link": cid },
                        "mimeType": mime,
                        "size": size,
                    },
                    "mime": mime,
                })
            }).collect())
            .unwrap_or_default();
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::WORK,
            "title": draft.title,
            "description": draft.summary,
            "tags": tags,
            "content": {
                "entry": manifest.get("entry").cloned().unwrap_or(serde_json::Value::Null),
                "contentFormat": draft.content_format.as_str(),
                "files": files,
            },
            "createdAt": now_rfc3339(),
        });
        let rkey = at_uri.rsplit('/').next().map(str::to_string);
        pds_create_record(&state, &user.token, fx_atproto::lexicon::WORK, record, rkey, "publish article").await;
    }
    if draft.at_uri.is_some() {
        pds_delete_record(&state, &user.token, fx_atproto::lexicon::DRAFT, id.clone(), "delete published draft").await;
    }

    let article = article_service::get_article_any_visibility(&state.pool, &at_uri).await?;
    Ok((StatusCode::CREATED, Json(article)))
}
