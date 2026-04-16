use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use fx_core::util::now_rfc3339;

use crate::auth::{Auth, pds_create_record};
use crate::error::{AppError, ApiResult};
use crate::state::AppState;

/// Start ORCID OAuth flow. Redirects the user's browser to ORCID authorization page.
/// The frontend should open this URL in a popup or new tab.
pub async fn orcid_start(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> Result<impl IntoResponse, AppError> {
    let client_id = state.orcid_client_id.as_deref()
        .ok_or(AppError(fx_core::Error::BadRequest("ORCID integration not configured".into())))?;

    let redirect_uri = format!("{}/api/orcid/callback", state.public_url.trim_end_matches('/'));

    let auth_url = format!(
        "https://orcid.org/oauth/authorize?client_id={}&response_type=code&scope=/authenticate&redirect_uri={}&state={}",
        urlencoding::encode(client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&user.token),
    );

    Ok(Redirect::temporary(&auth_url))
}

#[derive(serde::Deserialize)]
pub struct OrcidCallbackQuery {
    code: String,
    state: String, // bearer token to identify the user
}

/// ORCID OAuth callback. Exchanges the code for an ORCID iD, stores it, and creates a PDS record.
pub async fn orcid_callback(
    State(state): State<AppState>,
    Query(q): Query<OrcidCallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    let client_id = state.orcid_client_id.as_deref()
        .ok_or(AppError(fx_core::Error::BadRequest("ORCID integration not configured".into())))?;
    let client_secret = state.orcid_client_secret.as_deref()
        .ok_or(AppError(fx_core::Error::BadRequest("ORCID integration not configured".into())))?;

    let redirect_uri = format!("{}/api/orcid/callback", state.public_url.trim_end_matches('/'));

    // Exchange authorization code for access token + ORCID iD
    let resp = reqwest::Client::new()
        .post("https://orcid.org/oauth/token")
        .header("accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "authorization_code"),
            ("code", &q.code),
            ("redirect_uri", &redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| AppError(fx_core::Error::Internal(format!("ORCID token exchange failed: {e}"))))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError(fx_core::Error::Internal(format!("ORCID rejected token exchange: {body}"))));
    }

    let body: serde_json::Value = resp.json().await
        .map_err(|e| AppError(fx_core::Error::Internal(format!("ORCID response parse error: {e}"))))?;

    let orcid_id = body["orcid"].as_str()
        .ok_or(AppError(fx_core::Error::Internal("No ORCID iD in response".into())))?;

    // Look up the user from the bearer token passed as state
    let user = crate::auth::extract_auth_user_by_token(&state.pool, &q.state).await
        .ok_or(AppError(fx_core::Error::Unauthorized))?;

    // Store ORCID in profiles table
    sqlx::query(
        "UPDATE profiles SET orcid = $1, orcid_verified_at = NOW() WHERE did = $2",
    )
    .bind(orcid_id)
    .bind(&user.did)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError(fx_core::Error::Internal(format!("Failed to store ORCID: {e}"))))?;

    // Create PDS record for federated verification
    let record = serde_json::json!({
        "$type": fx_atproto::lexicon::ORCID,
        "orcid": orcid_id,
        "verifiedAt": now_rfc3339(),
    });
    let _ = pds_create_record(
        &state, &user.token, fx_atproto::lexicon::ORCID, record, Some("self".into()), "orcid binding",
    ).await;

    // Redirect to a success page
    let success_url = format!("{}/settings?orcid=linked", state.public_url.trim_end_matches('/'));
    Ok(Redirect::temporary(&success_url))
}

/// Get the current user's ORCID binding.
pub async fn get_orcid(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<serde_json::Value>> {
    let row: Option<(Option<String>, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT orcid, orcid_verified_at FROM profiles WHERE did = $1",
    )
    .bind(&user.did)
    .fetch_optional(&state.pool)
    .await?;

    let (orcid, verified_at) = row.unwrap_or((None, None));

    Ok(Json(serde_json::json!({
        "orcid": orcid,
        "verified_at": verified_at,
    })))
}

/// Unlink ORCID from the current user's profile.
pub async fn unlink_orcid(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<StatusCode> {
    sqlx::query(
        "UPDATE profiles SET orcid = NULL, orcid_verified_at = NULL WHERE did = $1",
    )
    .bind(&user.did)
    .execute(&state.pool)
    .await?;

    // TODO: delete PDS record too (need to know the rkey)

    Ok(StatusCode::NO_CONTENT)
}
