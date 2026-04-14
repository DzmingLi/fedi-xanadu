//! Authentication extractors and helpers.
//!
//! Provides Axum extractors for typed authentication:
//! - [`Auth`] — requires a valid session (rejects 401)
//! - [`WriteAuth`] — requires auth + write permission (rejects banned / unverified)
//! - [`MaybeAuth`] — optional auth (never rejects)

use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, request::Parts},
};
use fx_core::services::auth_service;

use crate::error::AppError;
use crate::state::AppState;

/// Authenticated user identity extracted from the session token.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub did: String,
    pub token: String,
    pub banned: bool,
    pub phone_verified: bool,
}

/// Requires authentication. Returns 401 if no valid session.
pub struct Auth(pub AuthUser);

impl FromRequestParts<AppState> for Auth {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match extract_auth_user(&state.pool, &parts.headers).await {
            Some(user) => Ok(Auth(user)),
            None => Err(AppError(fx_core::Error::Unauthorized)),
        }
    }
}

/// Requires authentication + permission to write.
/// Rejects banned users (403) and, on CN instances, users without phone verification.
pub struct WriteAuth(pub AuthUser);

impl FromRequestParts<AppState> for WriteAuth {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = extract_auth_user(&state.pool, &parts.headers)
            .await
            .ok_or(AppError(fx_core::Error::Unauthorized))?;

        if user.banned {
            return Err(AppError(fx_core::Error::Forbidden {
                action: "account is banned",
            }));
        }

        if state.instance_mode.requires_phone() && !user.phone_verified {
            return Err(AppError(fx_core::Error::Forbidden {
                action: "phone verification required",
            }));
        }

        Ok(WriteAuth(user))
    }
}

/// Optional authentication. Returns `None` if no valid session.
pub struct MaybeAuth(pub Option<AuthUser>);

impl FromRequestParts<AppState> for MaybeAuth {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        Ok(MaybeAuth(extract_auth_user(&state.pool, &parts.headers).await))
    }
}

async fn extract_auth_user(pool: &sqlx::PgPool, headers: &HeaderMap) -> Option<AuthUser> {
    // Try Bearer token first (platform-local sessions), then cookie (OAuth sessions)
    let token = extract_bearer_token(headers)
        .map(|t| t.to_string())
        .or_else(|| {
            headers.get_all("cookie")
                .iter()
                .filter_map(|v| v.to_str().ok())
                .flat_map(|s| s.split(';'))
                .find_map(|c| {
                    let c = c.trim();
                    c.strip_prefix("pad_session=").map(|v| v.to_string())
                })
        })?;
    let token_ref = token.as_str();

    // Check legacy sessions table first, then oauth_sessions
    let did = match auth_service::get_did_by_token(pool, token_ref).await.ok()? {
        Some(d) => d,
        None => {
            // Try oauth_sessions
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT did FROM oauth_sessions WHERE token = $1 AND expires_at > NOW()"
            ).bind(token_ref).fetch_optional(pool).await.ok()?;
            row?.0
        }
    };

    // Fetch ban status and phone verification in one query
    let row: Option<(bool, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT COALESCE(is_banned, false), phone_verified_at FROM platform_users WHERE did = $1",
    )
    .bind(&did)
    .fetch_optional(pool)
    .await
    .ok()?;

    let (banned, phone_verified) = match row {
        Some((b, pv)) => (b, pv.is_some()),
        None => (false, false), // AT Protocol user without platform_users row
    };

    Some(AuthUser {
        did,
        token,
        banned,
        phone_verified,
    })
}

pub fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    Some(auth.strip_prefix("Bearer ").unwrap_or(auth))
}

/// Get PDS session details for AT Protocol side-effects.
/// Returns `None` for platform-local users (no PDS).
pub async fn pds_session(
    pool: &sqlx::PgPool,
    token: &str,
) -> Option<auth_service::PdsSession> {
    let session = auth_service::get_session_for_pds(pool, token).await.ok()??;
    if session.pds_url.is_empty() {
        return None;
    }
    Some(session)
}

/// Log PDS sync failures without blocking the request.
pub fn log_pds_error<E: std::fmt::Display>(op: &str, e: E) {
    tracing::warn!("PDS sync failed ({op}): {e}");
}

/// Create a record on the user's PDS. Returns the record URI on success, or None.
pub async fn pds_create_record(
    state: &crate::state::AppState,
    token: &str,
    collection: &str,
    record: serde_json::Value,
    rkey: Option<String>,
    label: &str,
) -> Option<String> {
    let pds = pds_session(&state.pool, token).await?;
    match state.at_client.create_record(
        &pds.pds_url,
        &pds.access_jwt,
        &fx_atproto::client::CreateRecordInput {
            repo: pds.did,
            collection: collection.to_string(),
            record,
            rkey,
        },
    ).await {
        Ok(output) => Some(output.uri),
        Err(e) => { log_pds_error(label, e); None }
    }
}

/// Delete a record from the user's PDS.
pub async fn pds_delete_record(
    state: &crate::state::AppState,
    token: &str,
    collection: &str,
    rkey: String,
    label: &str,
) {
    if let Some(pds) = pds_session(&state.pool, token).await {
        if let Err(e) = state.at_client.delete_record(
            &pds.pds_url,
            &pds.access_jwt,
            &fx_atproto::client::DeleteRecordInput {
                repo: pds.did,
                collection: collection.to_string(),
                rkey,
            },
        ).await {
            log_pds_error(label, e);
        }
    }
}


/// Requires admin secret header. Returns 401/403 if invalid.
pub struct AdminAuth;

impl FromRequestParts<AppState> for AdminAuth {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let secret = state.admin_secret.as_deref()
            .ok_or(AppError(fx_core::Error::Forbidden { action: "admin not configured" }))?;
        let provided = parts.headers.get("x-admin-secret")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError(fx_core::Error::Unauthorized))?;
        if provided != secret {
            return Err(AppError(fx_core::Error::Unauthorized));
        }
        Ok(AdminAuth)
    }
}

/// Newtype for pijul-knot PadUser trait (orphan rules).
pub struct PadAuthUser(pub AuthUser);

impl pijul_knot::PadUser for PadAuthUser {
    fn did(&self) -> &str {
        &self.0.did
    }
}

pub struct PadAuthRejection;

impl axum::response::IntoResponse for PadAuthRejection {
    fn into_response(self) -> axum::response::Response {
        crate::error::AppError(fx_core::Error::Unauthorized).into_response()
    }
}

impl axum::extract::FromRequestParts<AppState> for PadAuthUser {
    type Rejection = PadAuthRejection;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match extract_auth_user(&state.pool, &parts.headers).await {
            Some(user) => Ok(PadAuthUser(user)),
            None => Err(PadAuthRejection),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_bearer_token_with_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer mytoken123".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), Some("mytoken123"));
    }

    #[test]
    fn extract_bearer_token_without_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "rawtoken".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), Some("rawtoken"));
    }

    #[test]
    fn extract_bearer_token_missing() {
        let headers = HeaderMap::new();
        assert_eq!(extract_bearer_token(&headers), None);
    }
}
