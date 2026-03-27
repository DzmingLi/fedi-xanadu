use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use fx_core::services::appeal_service;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use super::{Auth, tid};

const MAX_APPEAL_REASON: usize = 2000;

#[derive(serde::Deserialize)]
pub struct CreateAppealInput {
    pub kind: String,
    pub target_uri: Option<String>,
    pub reason: String,
}

/// Submit an appeal. Uses Auth (not WriteAuth) so banned users can appeal.
pub async fn create_appeal(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<CreateAppealInput>,
) -> ApiResult<(StatusCode, Json<appeal_service::Appeal>)> {
    if input.reason.is_empty() || input.reason.len() > MAX_APPEAL_REASON {
        return Err(AppError(fx_core::Error::BadRequest(
            format!("appeal reason must be 1-{MAX_APPEAL_REASON} characters"),
        )));
    }
    if input.kind != "ban" && input.kind != "article_deleted" {
        return Err(AppError(fx_core::Error::BadRequest(
            "kind must be 'ban' or 'article_deleted'".to_string(),
        )));
    }

    let id = tid();
    let appeal = appeal_service::create_appeal(
        &state.pool,
        &id,
        &user.did,
        &input.kind,
        input.target_uri.as_deref(),
        &input.reason,
    ).await?;

    Ok((StatusCode::CREATED, Json(appeal)))
}

/// List my appeals. Uses Auth so banned users can check appeal status.
pub async fn list_my_appeals(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<appeal_service::Appeal>>> {
    let appeals = appeal_service::list_my_appeals(&state.pool, &user.did).await?;
    Ok(Json(appeals))
}
