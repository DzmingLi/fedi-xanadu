use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use fx_core::services::{auth_service, platform_user_service};

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use super::{Auth, extract_bearer_token, gen_session_token};

#[derive(serde::Deserialize)]
pub struct LoginInput {
    identifier: String,
    password: String,
}

#[derive(serde::Serialize)]
pub struct LoginOutput {
    token: String,
    did: String,
    handle: String,
    display_name: Option<String>,
    avatar: Option<String>,
}

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> ApiResult<Json<LoginOutput>> {
    // Try platform-local user first
    if let Some(platform_user) = platform_user_service::get_by_handle(&state.pool, &input.identifier).await? {
        let password = input.password.clone();
        let hash = platform_user.password_hash.clone();
        let valid = tokio::task::spawn_blocking(move || {
            platform_user_service::verify_password(&password, &hash).unwrap_or(false)
        }).await.unwrap_or(false);

        if !valid {
            return Err(AppError(fx_core::Error::Unauthorized));
        }

        let token = gen_session_token();

        // Fetch display info from profiles table
        let profile: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT display_name, avatar_url FROM profiles WHERE did = $1",
        )
        .bind(&platform_user.did)
        .fetch_optional(&state.pool)
        .await?;

        let (display_name, avatar) = profile.unwrap_or((None, None));

        auth_service::create_session(
            &state.pool,
            &auth_service::CreateSessionInput {
                token: &token,
                did: &platform_user.did,
                handle: &platform_user.handle,
                display_name: display_name.as_deref(),
                avatar: avatar.as_deref(),
                pds_url: "",
                access_jwt: "",
                refresh_jwt: None,
            },
        ).await?;

        return Ok(Json(LoginOutput {
            token,
            did: platform_user.did,
            handle: platform_user.handle,
            display_name,
            avatar,
        }));
    }

    // Fall through to AT Protocol login
    let (did, pds_url) = state
        .at_client
        .resolve_handle(&input.identifier)
        .await
        .map_err(|e| AppError(fx_core::Error::BadRequest(format!("Cannot resolve handle: {e}"))))?;

    let session = state
        .at_client
        .create_session(&pds_url, &input.identifier, &input.password)
        .await
        .map_err(|_| AppError(fx_core::Error::Unauthorized))?;

    let profile = state
        .at_client
        .get_profile(&pds_url, &did, &session.access_jwt)
        .await
        .ok();

    let display_name = profile
        .as_ref()
        .and_then(|p| p.display_name.clone())
        .or(session.display_name.clone());
    let avatar = profile
        .as_ref()
        .and_then(|p| p.avatar.clone())
        .or(session.avatar.clone());

    let token = gen_session_token();

    auth_service::create_session(
        &state.pool,
        &auth_service::CreateSessionInput {
            token: &token,
            did: &did,
            handle: &session.handle,
            display_name: display_name.as_deref(),
            avatar: avatar.as_deref(),
            pds_url: &pds_url,
            access_jwt: &session.access_jwt,
            refresh_jwt: session.refresh_jwt.as_deref(),
        },
    )
    .await?;

    let _ = auth_service::upsert_profile(
        &state.pool,
        &did,
        &session.handle,
        display_name.as_deref(),
        avatar.as_deref(),
    )
    .await;

    Ok(Json(LoginOutput {
        token,
        did,
        handle: session.handle,
        display_name,
        avatar,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    if let Some(token) = extract_bearer_token(&headers) {
        let _ = auth_service::delete_session(&state.pool, token).await;
    }
    Ok(StatusCode::OK)
}

#[derive(serde::Serialize)]
pub(crate) struct AuthMeOutput {
    did: String,
    handle: String,
    display_name: Option<String>,
    avatar: Option<String>,
}

pub async fn auth_me(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<AuthMeOutput>> {
    let session = auth_service::get_session_by_token(&state.pool, &user.token)
        .await?
        .ok_or(AppError(fx_core::Error::Unauthorized))?;

    Ok(Json(AuthMeOutput {
        did: session.did,
        handle: session.handle,
        display_name: session.display_name,
        avatar: session.avatar_url,
    }))
}
