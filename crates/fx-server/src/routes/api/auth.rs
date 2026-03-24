use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use super::gen_session_token;

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
    let (did, pds_url) = state
        .at_client
        .resolve_handle(&input.identifier)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Cannot resolve handle: {e}")))?;

    let session = state
        .at_client
        .create_session(&pds_url, &input.identifier, &input.password)
        .await
        .map_err(|_| ApiError::Unauthorized)?;

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

    sqlx::query(
        "INSERT OR REPLACE INTO sessions (token, did, handle, display_name, avatar_url, pds_url, access_jwt, refresh_jwt)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&token)
    .bind(&did)
    .bind(&session.handle)
    .bind(&display_name)
    .bind(&avatar)
    .bind(&pds_url)
    .bind(&session.access_jwt)
    .bind(&session.refresh_jwt)
    .execute(&state.pool)
    .await?;

    let _ = sqlx::query(
        "INSERT INTO profiles (did, handle, display_name, avatar_url) VALUES (?, ?, ?, ?)
         ON CONFLICT(did) DO UPDATE SET handle = excluded.handle, display_name = excluded.display_name,
         avatar_url = excluded.avatar_url, updated_at = datetime('now')",
    )
    .bind(&did)
    .bind(&session.handle)
    .bind(&display_name)
    .bind(&avatar)
    .execute(&state.pool)
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
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        let token = auth.strip_prefix("Bearer ").unwrap_or(auth);
        let _ = sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&state.pool)
            .await;
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
    headers: HeaderMap,
) -> ApiResult<Json<AuthMeOutput>> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    let token = auth.strip_prefix("Bearer ").unwrap_or(auth);

    #[derive(sqlx::FromRow)]
    struct Row {
        did: String,
        handle: String,
        display_name: Option<String>,
        avatar_url: Option<String>,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT did, handle, display_name, avatar_url FROM sessions WHERE token = ? AND expires_at > datetime('now')",
    )
    .bind(token)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    Ok(Json(AuthMeOutput {
        did: row.did,
        handle: row.handle,
        display_name: row.display_name,
        avatar: row.avatar_url,
    }))
}
