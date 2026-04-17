use axum::{
    Json,
    body::Body,
    extract::{Multipart, Query, State},
    http::{StatusCode, Response, header},
};
use fx_core::services::social_service;

use crate::avatar_cache;
use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::WriteAuth;
use super::DidQuery;

#[derive(serde::Deserialize)]
pub(crate) struct UpdateProfileContactsInput {
    contacts: social_service::Contacts,
}

pub async fn get_profile(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<social_service::ProfileResponse>> {
    let mut profile = social_service::get_profile(&state.pool, &did).await?;

    // Sync profile from PDS for AT Protocol users. Cache the remote avatar
    // locally so subsequent pageviews hit /api/avatars/{did} instead of the
    // Bluesky CDN.
    if did.starts_with("did:plc:") || did.starts_with("did:web:") {
        if let Ok(bsky) = state.at_client.get_public_profile(&did).await {
            let cached_avatar = if let Some(remote) = bsky.avatar.as_deref() {
                avatar_cache::cache_remote_avatar(&state.data_dir, &did, remote).await
            } else {
                None
            };
            let effective_avatar = cached_avatar.clone().or_else(|| bsky.avatar.clone());

            let pool = state.pool.clone();
            let did_clone = did.clone();
            let dn = bsky.display_name.clone();
            let av = effective_avatar.clone();
            let banner = bsky.banner.clone();
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "UPDATE profiles SET display_name = $1, avatar_url = $2, banner_url = COALESCE($3, banner_url) WHERE did = $4"
                )
                .bind(&dn)
                .bind(&av)
                .bind(&banner)
                .bind(&did_clone)
                .execute(&pool).await;
            });

            profile.display_name = bsky.display_name;
            profile.avatar_url = effective_avatar;
            if bsky.banner.is_some() { profile.banner_url = bsky.banner; }
        }
    }

    Ok(Json(profile))
}

pub async fn update_profile_contacts(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UpdateProfileContactsInput>,
) -> ApiResult<StatusCode> {
    social_service::update_profile_contacts(&state.pool, &user.did, &input.contacts).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub(crate) struct UpdateBioInput {
    bio: String,
}

pub async fn update_bio(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UpdateBioInput>,
) -> ApiResult<StatusCode> {
    sqlx::query("UPDATE profiles SET bio = $1 WHERE did = $2")
        .bind(&input.bio)
        .bind(&user.did)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub(crate) struct UpdateDisplayNameInput {
    display_name: String,
}

pub async fn update_display_name(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<UpdateDisplayNameInput>,
) -> ApiResult<StatusCode> {
    sqlx::query("UPDATE profiles SET display_name = $1 WHERE did = $2")
        .bind(&input.display_name)
        .bind(&user.did)
        .execute(&state.pool)
        .await?;

    // Sync to PDS (best-effort)
    let _ = crate::auth::pds_update_profile(
        &state, &user.token, Some(&input.display_name), None, None,
    ).await;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_education(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<Vec<social_service::EducationEntry>>,
) -> ApiResult<StatusCode> {
    let json = serde_json::to_value(&input)?;
    sqlx::query("UPDATE profiles SET education = $1 WHERE did = $2")
        .bind(&json).bind(&user.did).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_publications(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<Vec<social_service::PublicationEntry>>,
) -> ApiResult<StatusCode> {
    let json = serde_json::to_value(&input)?;
    sqlx::query("UPDATE profiles SET publications = $1 WHERE did = $2")
        .bind(&json).bind(&user.did).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_projects(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<Vec<social_service::ProjectEntry>>,
) -> ApiResult<StatusCode> {
    let json = serde_json::to_value(&input)?;
    sqlx::query("UPDATE profiles SET projects = $1 WHERE did = $2")
        .bind(&json).bind(&user.did).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_teaching(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<Vec<social_service::TeachingEntry>>,
) -> ApiResult<StatusCode> {
    let json = serde_json::to_value(&input)?;
    sqlx::query("UPDATE profiles SET teaching = $1 WHERE did = $2")
        .bind(&json).bind(&user.did).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Avatar upload & serve ---

const AVATAR_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const MAX_AVATAR_SIZE: usize = 2 * 1024 * 1024; // 2 MB

pub async fn upload_avatar(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    mut multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError(fx_core::Error::BadRequest(format!("Multipart error: {e}")))
    })? {
        if field.name() == Some("file") {
            file_name = field.file_name().map(|s| s.to_string());
            file_data = Some(field.bytes().await
                .map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?.to_vec());
        }
    }

    let data = file_data.ok_or(AppError(fx_core::Error::BadRequest("Missing file".into())))?;
    if data.len() > MAX_AVATAR_SIZE {
        return Err(AppError(fx_core::Error::BadRequest("Avatar too large (max 2MB)".into())));
    }

    let ext = file_name.as_deref()
        .and_then(|n| std::path::Path::new(n).extension())
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| "jpg".into());
    if !AVATAR_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest("Use jpg, png, or webp".into())));
    }

    let safe_did: String = user.did.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ':')
        .collect();

    let avatars_dir = state.data_dir.join("avatars");
    let _ = tokio::fs::create_dir_all(&avatars_dir).await;
    tokio::fs::write(avatars_dir.join(format!("{safe_did}.{ext}")), &data).await?;

    let content_type = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };

    let avatar_url = format!("/api/avatars/{safe_did}");
    sqlx::query("UPDATE profiles SET avatar_url = $1 WHERE did = $2")
        .bind(&avatar_url).bind(&user.did).execute(&state.pool).await?;

    // Sync avatar to PDS (best-effort)
    if let Some(blob) = crate::auth::pds_upload_blob(&state, &user.token, data, content_type).await {
        let _ = crate::auth::pds_update_profile(&state, &user.token, None, Some(blob), None).await;
    }

    Ok(Json(serde_json::json!({ "avatar_url": avatar_url })))
}

const BANNER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const MAX_BANNER_SIZE: usize = 5 * 1024 * 1024; // 5 MB

pub async fn upload_banner(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    mut multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError(fx_core::Error::BadRequest(format!("Multipart error: {e}")))
    })? {
        if field.name() == Some("file") {
            file_name = field.file_name().map(|s| s.to_string());
            file_data = Some(field.bytes().await
                .map_err(|e| AppError(fx_core::Error::BadRequest(e.to_string())))?.to_vec());
        }
    }

    let data = file_data.ok_or(AppError(fx_core::Error::BadRequest("Missing file".into())))?;
    if data.len() > MAX_BANNER_SIZE {
        return Err(AppError(fx_core::Error::BadRequest("Banner too large (max 5MB)".into())));
    }

    let ext = file_name.as_deref()
        .and_then(|n| std::path::Path::new(n).extension())
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| "jpg".into());
    if !BANNER_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError(fx_core::Error::BadRequest("Use jpg, png, or webp".into())));
    }

    let safe_did: String = user.did.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ':')
        .collect();

    let banners_dir = state.data_dir.join("banners");
    let _ = tokio::fs::create_dir_all(&banners_dir).await;
    tokio::fs::write(banners_dir.join(format!("{safe_did}.{ext}")), &data).await?;

    let content_type = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };

    let banner_url = format!("/api/banners/{safe_did}");
    sqlx::query("UPDATE profiles SET banner_url = $1 WHERE did = $2")
        .bind(&banner_url).bind(&user.did).execute(&state.pool).await?;

    // Sync banner to PDS (best-effort)
    if let Some(blob) = crate::auth::pds_upload_blob(&state, &user.token, data, content_type).await {
        let _ = crate::auth::pds_update_profile(&state, &user.token, None, None, Some(blob)).await;
    }

    Ok(Json(serde_json::json!({ "banner_url": banner_url })))
}

pub async fn get_banner(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response<Body> {
    let safe_id: String = id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ':')
        .collect();

    let banners_dir = state.data_dir.join("banners");
    for ext in BANNER_EXTENSIONS {
        let path = banners_dir.join(format!("{safe_id}.{ext}"));
        if path.exists() {
            let ct = match *ext { "png" => "image/png", "webp" => "image/webp", _ => "image/jpeg" };
            if let Ok(data) = tokio::fs::read(&path).await {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, ct)
                    .header(header::CACHE_CONTROL, "public, max-age=3600")
                    .body(Body::from(data)).unwrap();
            }
        }
    }
    Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()
}

pub async fn get_avatar(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response<Body> {
    let safe_id: String = id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ':')
        .collect();

    let avatars_dir = state.data_dir.join("avatars");
    for ext in AVATAR_EXTENSIONS {
        let path = avatars_dir.join(format!("{safe_id}.{ext}"));
        if path.exists() {
            let ct = match *ext { "png" => "image/png", "webp" => "image/webp", _ => "image/jpeg" };
            if let Ok(data) = tokio::fs::read(&path).await {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, ct)
                    .header(header::CACHE_CONTROL, "public, max-age=3600")
                    .body(Body::from(data)).unwrap();
            }
        }
    }
    Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()
}

pub async fn get_user_listings(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<Vec<fx_core::services::listing_service::Listing>>> {
    let listings = fx_core::services::listing_service::list_my_listings(&state.pool, &did).await?;
    let open: Vec<_> = listings.into_iter().filter(|l| l.is_open).collect();
    Ok(Json(open))
}
