use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use fx_core::services::{auth_service, platform_user_service};

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::{Auth, extract_bearer_token};
use fx_core::util::gen_session_token;

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

/// Platform-local user login only. AT Protocol users use OAuth at /oauth/login.
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> ApiResult<Json<LoginOutput>> {
    let platform_user = platform_user_service::get_by_handle(&state.pool, &input.identifier)
        .await?
        .ok_or(AppError(fx_core::Error::Unauthorized))?;

    let password = input.password.clone();
    let hash = platform_user.password_hash.clone();
    let valid = tokio::task::spawn_blocking(move || {
        platform_user_service::verify_password(&password, &hash).unwrap_or(false)
    }).await.unwrap_or(false);

    if !valid {
        return Err(AppError(fx_core::Error::Unauthorized));
    }

    let token = gen_session_token();

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

    Ok(Json(LoginOutput {
        token,
        did: platform_user.did,
        handle: platform_user.handle,
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
    is_banned: bool,
    ban_reason: Option<String>,
}

pub async fn auth_me(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<AuthMeOutput>> {
    // Try legacy session first, then oauth_sessions
    let (did, handle, display_name, avatar) =
        if let Some(session) = auth_service::get_session_by_token(&state.pool, &user.token).await? {
            (session.did, session.handle, session.display_name, session.avatar_url)
        } else {
            // OAuth session — get handle from oauth_sessions, profile from profiles table
            let row: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT did, handle FROM oauth_sessions WHERE token = $1 AND expires_at > NOW()"
            ).bind(&user.token).fetch_optional(&state.pool).await?;
            let (did, handle) = row.ok_or(AppError(fx_core::Error::Unauthorized))?;
            let handle = handle.unwrap_or_default();
            let profile: Option<(Option<String>, Option<String>)> = sqlx::query_as(
                "SELECT display_name, avatar_url FROM profiles WHERE did = $1"
            ).bind(&did).fetch_optional(&state.pool).await?;
            let (dn, av) = profile.unwrap_or((None, None));
            (did, handle, dn, av)
        };

    let (is_banned, ban_reason) = if user.banned {
        let reason: Option<String> = sqlx::query_scalar(
            "SELECT ban_reason FROM platform_users WHERE did = $1",
        )
        .bind(&user.did)
        .fetch_optional(&state.pool)
        .await?
        .flatten();
        (true, reason)
    } else {
        (false, None)
    };

    Ok(Json(AuthMeOutput {
        did,
        handle,
        display_name,
        avatar,
        is_banned,
        ban_reason,
    }))
}
