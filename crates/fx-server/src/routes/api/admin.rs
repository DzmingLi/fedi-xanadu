use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use fx_core::services::platform_user_service;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;

fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    let secret = state.admin_secret.as_deref()
        .ok_or(AppError(fx_core::Error::Forbidden { action: "admin not configured" }))?;
    let provided = headers.get("x-admin-secret")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError(fx_core::Error::Unauthorized))?;
    if provided != secret {
        return Err(AppError(fx_core::Error::Unauthorized));
    }
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct CreatePlatformUserInput {
    handle: String,
    password: String,
    display_name: Option<String>,
}

pub async fn create_platform_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreatePlatformUserInput>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers)?;

    let did = platform_user_service::create_platform_user(
        &state.pool,
        &input.handle,
        input.display_name.as_deref(),
        &input.password,
    ).await?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "did": did,
        "handle": input.handle,
    }))))
}

pub async fn list_platform_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<platform_user_service::PlatformUserInfo>>> {
    require_admin(&state, &headers)?;
    let users = platform_user_service::list_platform_users(&state.pool).await?;
    Ok(Json(users))
}
