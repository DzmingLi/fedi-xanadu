use axum::{Json, extract::State};
use fx_core::services::report_service;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::WriteAuth;

#[derive(serde::Deserialize)]
pub struct CreateReportInput {
    pub target_did: String,
    pub target_uri: Option<String>,
    pub kind: String,
    pub reason: String,
}

#[utoipa::path(post, path = "/api/v1/reports", responses((status = 200)), security(("bearer" = [])))]
pub async fn create_report(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateReportInput>,
) -> ApiResult<Json<report_service::Report>> {
    let id = fx_core::util::tid();
    let report = report_service::create_report(
        &state.pool,
        &id,
        &user.did,
        &input.target_did,
        input.target_uri.as_deref(),
        &input.kind,
        &input.reason,
    )
    .await?;
    Ok(Json(report))
}
