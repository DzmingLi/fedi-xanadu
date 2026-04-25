//! REST API for publications (专栏).
//!
//! Every mutating handler also syncs to the caller's PDS when they have a
//! session, so the publication graph (memberships, cross-post entries,
//! follows) is federation-visible. DB writes are the source of truth for
//! server-side queries; PDS writes are best-effort.
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fx_atproto::lexicon;
use fx_core::services::publication_service as svc;
use serde::Deserialize;

use crate::auth::{WriteAuth, pds_create_record, pds_delete_record};
use crate::error::{AppError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

pub async fn list_publications(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<svc::PublicationSummary>>> {
    let limit = q.limit.unwrap_or(50).min(100);
    let offset = q.offset.unwrap_or(0);
    let rows = svc::list_publications(&state.pool, limit, offset).await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CreateInput {
    pub id: String,
    pub title_i18n: svc::L,
    #[serde(default)]
    pub description_i18n: svc::L,
    #[serde(default)]
    pub cover_url: Option<String>,
}

pub async fn create_publication(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateInput>,
) -> ApiResult<Json<svc::Publication>> {
    let p = svc::create_publication(
        &state.pool,
        &user.did,
        &input.id,
        &input.title_i18n,
        &input.description_i18n,
        input.cover_url.as_deref(),
    )
    .await?;

    // PDS sync: owner's publication record
    let record = serde_json::json!({
        "$type": lexicon::PUBLICATION,
        "slug": p.id,
        "title": p.title_i18n.0,
        "description": p.description_i18n.0,
        "coverUrl": p.cover_url,
        "members": [{"did": user.did, "role": "owner"}],
        "createdAt": p.created_at.to_rfc3339(),
    });
    if let Some(uri) = pds_create_record(
        &state,
        &user.token,
        lexicon::PUBLICATION,
        record,
        Some(p.id.clone()),
        "create publication",
    )
    .await
    {
        svc::set_publication_at_uri(&state.pool, &p.id, &uri).await?;
    }

    Ok(Json(p))
}

pub async fn get_publication(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<svc::Publication>> {
    let p = svc::get_publication(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError(fx_core::Error::NotFound { entity: "publication", id: slug.clone() }))?;
    Ok(Json(p))
}

#[derive(Deserialize)]
pub struct UpdateInput {
    pub title_i18n: svc::L,
    #[serde(default)]
    pub description_i18n: svc::L,
    #[serde(default)]
    pub cover_url: Option<String>,
}

pub async fn update_publication(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(slug): Path<String>,
    Json(input): Json<UpdateInput>,
) -> ApiResult<StatusCode> {
    svc::update_publication(
        &state.pool,
        &user.did,
        &slug,
        &input.title_i18n,
        &input.description_i18n,
        input.cover_url.as_deref(),
    )
    .await?;
    // PDS sync: only owners would need to rewrite their root record; editors
    // don't own it. Skipped for MVP — the DB is authoritative and the owner
    // can re-sync via a future reconciler.
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_publication(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    svc::delete_publication(&state.pool, &user.did, &slug).await?;
    pds_delete_record(
        &state,
        &user.token,
        lexicon::PUBLICATION,
        slug.clone(),
        "delete publication",
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// ---- Members ----

pub async fn list_members(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<Vec<svc::PublicationMember>>> {
    Ok(Json(svc::list_members(&state.pool, &slug).await?))
}

#[derive(Deserialize)]
pub struct MemberInput {
    pub did: String,
    pub role: String,
}

pub async fn add_member(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(slug): Path<String>,
    Json(input): Json<MemberInput>,
) -> ApiResult<StatusCode> {
    let role = svc::Role::parse(&input.role)
        .ok_or_else(|| AppError(fx_core::Error::BadRequest("invalid role".into())))?;
    svc::add_member(&state.pool, &user.did, &slug, &input.did, role).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_member(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path((slug, did)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    svc::remove_member(&state.pool, &user.did, &slug, &did).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Member accepts an invitation by creating a membership record on their PDS.
/// The caller's DID must already be listed in the publication's members.
pub async fn accept_membership(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    let pub_ = svc::get_publication(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError(fx_core::Error::NotFound { entity: "publication", id: slug.clone() }))?;
    let role = svc::user_role(&state.pool, &slug, &user.did)
        .await?
        .ok_or_else(|| AppError(fx_core::Error::BadRequest("no pending invitation".into())))?;

    let record = serde_json::json!({
        "$type": lexicon::PUBLICATION_MEMBERSHIP,
        "publicationUri": pub_.at_uri,
        "publicationSlug": slug,
        "role": role.as_str(),
        "acceptedAt": chrono::Utc::now().to_rfc3339(),
    });
    if let Some(uri) = pds_create_record(
        &state,
        &user.token,
        lexicon::PUBLICATION_MEMBERSHIP,
        record,
        Some(slug.clone()),
        "accept publication membership",
    )
    .await
    {
        svc::confirm_membership(&state.pool, &slug, &user.did, &uri).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn leave_publication(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    svc::leave_publication(&state.pool, &user.did, &slug).await?;
    pds_delete_record(
        &state,
        &user.token,
        lexicon::PUBLICATION_MEMBERSHIP,
        slug,
        "leave publication",
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// ---- Content (cross-post) ----

#[derive(Deserialize)]
pub struct ContentInput {
    pub content_uri: String,
    pub content_kind: String,
}

pub async fn add_content(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(slug): Path<String>,
    Json(input): Json<ContentInput>,
) -> ApiResult<StatusCode> {
    svc::add_content(
        &state.pool,
        &user.did,
        &slug,
        &input.content_uri,
        &input.content_kind,
    )
    .await?;

    let pub_ = svc::get_publication(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError(fx_core::Error::NotFound { entity: "publication", id: slug.clone() }))?;
    let record = serde_json::json!({
        "$type": lexicon::PUBLICATION_ENTRY,
        "publicationUri": pub_.at_uri,
        "publicationSlug": slug,
        "contentUri": input.content_uri,
        "contentKind": input.content_kind,
        "createdAt": chrono::Utc::now().to_rfc3339(),
    });
    if let Some(uri) = pds_create_record(
        &state,
        &user.token,
        lexicon::PUBLICATION_ENTRY,
        record,
        None,
        "add publication entry",
    )
    .await
    {
        svc::set_entry_at_uri(&state.pool, &slug, &input.content_uri, &uri).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct RemoveContentInput {
    pub content_uri: String,
}

pub async fn remove_content(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(slug): Path<String>,
    Json(input): Json<RemoveContentInput>,
) -> ApiResult<StatusCode> {
    // Look up entry record URI before we delete the DB row.
    let entry_uri: Option<String> = sqlx::query_scalar(
        "SELECT entry_at_uri FROM publication_content WHERE publication_id = $1 AND content_uri = $2",
    )
    .bind(&slug)
    .bind(&input.content_uri)
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    svc::remove_content(&state.pool, &user.did, &slug, &input.content_uri).await?;

    if let Some(uri) = entry_uri {
        // Parse rkey out of at://did/collection/rkey
        if let Some(rkey) = uri.rsplit('/').next() {
            pds_delete_record(
                &state,
                &user.token,
                lexicon::PUBLICATION_ENTRY,
                rkey.to_string(),
                "remove publication entry",
            )
            .await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_content(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<svc::PublicationContentItem>>> {
    let limit = q.limit.unwrap_or(50).min(100);
    let offset = q.offset.unwrap_or(0);
    let rows = svc::list_content_uris(&state.pool, &slug, limit, offset).await?;

    let article_uris: Vec<String> = rows.iter()
        .filter(|(_, kind, _)| kind == "article")
        .map(|(u, _, _)| u.clone())
        .collect();
    let series_ids: Vec<String> = rows.iter()
        .filter(|(_, kind, _)| kind == "series")
        .map(|(u, _, _)| u.clone())
        .collect();

    let articles = if article_uris.is_empty() {
        Vec::new()
    } else {
        fx_core::services::article_service::get_articles_by_uris(
            &state.pool,
            state.instance_mode,
            &article_uris,
        )
        .await?
    };
    let article_map: std::collections::HashMap<String, fx_core::models::Article> =
        articles
            .into_iter()
            .filter_map(|a| a.at_uri.clone().map(|u| (u, a)))
            .collect();

    let series_rows = if series_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, fx_core::services::series_service::SeriesListRow>(
            "SELECT s.id, s.title, s.summary, s.long_description, s.order_index, \
                    s.created_by, p.handle AS author_handle, p.display_name AS author_display_name, \
                    p.avatar_url AS author_avatar, s.created_at, s.lang, s.translation_group, \
                    s.category, s.split_level, s.is_published, \
                    COALESCE((SELECT SUM(value) FROM votes WHERE target_uri = s.id), 0) AS vote_score, \
                    COALESCE((SELECT COUNT(*) FROM user_bookmarks WHERE article_uri = s.id), 0) AS bookmark_count \
             FROM series s \
             LEFT JOIN profiles p ON p.did = s.created_by \
             WHERE s.id = ANY($1)",
        )
        .bind(&series_ids)
        .fetch_all(&state.pool)
        .await?
    };
    let series_map: std::collections::HashMap<String, fx_core::services::series_service::SeriesListRow> =
        series_rows.into_iter().map(|s| (s.id.clone(), s)).collect();

    let items: Vec<svc::PublicationContentItem> = rows
        .into_iter()
        .filter_map(|(uri, kind, added_at)| match kind.as_str() {
            "article" => article_map.get(&uri).cloned().map(|a| svc::PublicationContentItem {
                kind,
                added_at,
                article: Some(a),
                series: None,
            }),
            "series" => series_map.get(&uri).cloned().map(|s| svc::PublicationContentItem {
                kind,
                added_at,
                article: None,
                series: Some(s),
            }),
            _ => None,
        })
        .collect();

    Ok(Json(items))
}

// ---- Follow ----

pub async fn follow_publication(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    let pub_ = svc::get_publication(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError(fx_core::Error::NotFound { entity: "publication", id: slug.clone() }))?;
    let record = serde_json::json!({
        "$type": lexicon::PUBLICATION_FOLLOW,
        "publicationUri": pub_.at_uri,
        "publicationSlug": slug,
        "createdAt": chrono::Utc::now().to_rfc3339(),
    });
    let at_uri = pds_create_record(
        &state,
        &user.token,
        lexicon::PUBLICATION_FOLLOW,
        record,
        Some(slug.clone()),
        "follow publication",
    )
    .await;
    svc::follow(&state.pool, &user.did, &slug, at_uri.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unfollow_publication(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    svc::unfollow(&state.pool, &user.did, &slug).await?;
    pds_delete_record(
        &state,
        &user.token,
        lexicon::PUBLICATION_FOLLOW,
        slug,
        "unfollow publication",
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn viewer_state(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(slug): Path<String>,
) -> ApiResult<Json<svc::PublicationViewerState>> {
    let role = svc::user_role(&state.pool, &slug, &user.did).await?;
    let is_following = svc::is_following(&state.pool, &user.did, &slug).await?;
    let confirmed: Option<String> = sqlx::query_scalar(
        "SELECT membership_at_uri FROM publication_members WHERE publication_id = $1 AND did = $2",
    )
    .bind(&slug)
    .bind(&user.did)
    .fetch_optional(&state.pool)
    .await?
    .flatten();
    Ok(Json(svc::PublicationViewerState {
        role: role.map(|r| r.as_str().to_string()),
        is_following,
        membership_confirmed: confirmed.is_some(),
    }))
}

/// Publications the signed-in user has writer+ access to — drives the
/// "cross-post to" dropdown on the publish form.
pub async fn my_writable_publications(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
) -> ApiResult<Json<Vec<svc::Publication>>> {
    Ok(Json(svc::writable_publications_for(&state.pool, &user.did).await?))
}

pub async fn publications_for_content_handler(
    State(state): State<AppState>,
    Query(q): Query<ContentRefQuery>,
) -> ApiResult<Json<Vec<svc::Publication>>> {
    Ok(Json(svc::publications_for_content(&state.pool, &q.uri).await?))
}

#[derive(Deserialize)]
pub struct ContentRefQuery {
    pub uri: String,
}
