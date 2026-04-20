use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use fx_core::services::{auth_service, platform_user_service};

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::{Auth, extract_bearer_token};
use fx_core::util::{gen_session_token, now_rfc3339};

/// Append the PDS's default handle domain when the user typed only the local
/// part (no dots). e.g. "alice" on a pds.nightbo.at instance → "alice.nightbo.at".
fn normalize_handle(input: &str, pds_url: &str) -> String {
    let h = input.trim().to_lowercase();
    if h.contains('.') { return h; }
    let suffix = pds_url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("pds.")
        .trim_end_matches('/');
    if suffix.is_empty() { h } else { format!("{h}.{suffix}") }
}

#[derive(serde::Deserialize)]
struct PdsSession {
    #[serde(rename = "accessJwt")]
    access_jwt: String,
    #[serde(rename = "refreshJwt")]
    refresh_jwt: String,
    did: String,
    handle: String,
}

/// POST {pds}/xrpc/<method>, returning decoded JSON or a friendly error.
async fn pds_xrpc<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    pds_url: &str,
    method: &str,
    body: serde_json::Value,
) -> Result<T, AppError> {
    let url = format!("{}/xrpc/{method}", pds_url.trim_end_matches('/'));
    let resp = client.post(&url).json(&body).send().await
        .map_err(|e| AppError(fx_core::Error::AtProto(format!("PDS {method} request: {e}"))))?;
    if !resp.status().is_success() {
        let code = resp.status();
        let body = resp.text().await.unwrap_or_default();
        // Surface the PDS's structured error when we can parse one out.
        #[derive(serde::Deserialize)]
        struct PdsErr { error: Option<String>, message: Option<String> }
        let friendly = serde_json::from_str::<PdsErr>(&body).ok()
            .and_then(|e| e.message.or(e.error))
            .unwrap_or_else(|| body.chars().take(200).collect());
        return Err(AppError(match code.as_u16() {
            400 | 401 => fx_core::Error::BadRequest(friendly),
            _ => fx_core::Error::AtProto(format!("PDS {method}: {friendly}")),
        }));
    }
    resp.json::<T>().await.map_err(|e| AppError(fx_core::Error::AtProto(e.to_string())))
}

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

#[derive(serde::Deserialize)]
pub struct RegisterInput {
    handle: String,
    password: String,
    display_name: Option<String>,
}

/// Self-service registration: creates an account on the instance's home PDS.
/// Returns a session token tied to the new PDS account.
pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterInput>,
) -> ApiResult<(StatusCode, Json<LoginOutput>)> {
    if state.pds_url.is_empty() {
        return Err(AppError(fx_core::Error::BadRequest(
            "Registration disabled: this instance has no home PDS configured".into(),
        )));
    }

    let local = input.handle.trim().to_lowercase();
    if local.len() < 2 || local.len() > 32 {
        return Err(AppError(fx_core::Error::BadRequest("Handle must be 2-32 characters".into())));
    }
    if !local.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError(fx_core::Error::BadRequest("Handle may only contain a-z, 0-9, _ and -".into())));
    }
    if input.password.len() < 8 {
        return Err(AppError(fx_core::Error::BadRequest("Password must be at least 8 characters".into())));
    }

    let handle = normalize_handle(&local, &state.pds_url);
    let email = format!("{local}@{}",
        state.public_url
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/'));

    let client = reqwest::Client::new();
    let sess: PdsSession = pds_xrpc(&client, &state.pds_url, "com.atproto.server.createAccount",
        serde_json::json!({
            "email": email,
            "handle": handle,
            "password": input.password,
        }),
    ).await?;

    // Seed profile on the PDS so display_name shows up in bsky / NightBoat lists.
    if let Some(ref dn) = input.display_name {
        let profile = serde_json::json!({
            "$type": "app.bsky.actor.profile",
            "displayName": dn,
            "createdAt": now_rfc3339(),
        });
        let _ = pds_xrpc::<serde_json::Value>(&client, &state.pds_url, "com.atproto.repo.putRecord",
            serde_json::json!({
                "repo": sess.did,
                "collection": "app.bsky.actor.profile",
                "rkey": "self",
                "record": profile,
            }),
        ).await;
    }

    // Cache the profile locally too — AppView reads its profiles table, not the PDS.
    let _ = sqlx::query(
        "INSERT INTO profiles (did, handle, display_name) VALUES ($1, $2, $3) \
         ON CONFLICT (did) DO UPDATE SET handle = EXCLUDED.handle, display_name = EXCLUDED.display_name",
    )
    .bind(&sess.did)
    .bind(&sess.handle)
    .bind(input.display_name.as_deref())
    .execute(&state.pool)
    .await;

    // Store the PDS session keyed by our own opaque token (keeps the client
    // interface uniform between password login and OAuth).
    let token = gen_session_token();
    auth_service::create_session(
        &state.pool,
        &auth_service::CreateSessionInput {
            token: &token,
            did: &sess.did,
            handle: &sess.handle,
            display_name: input.display_name.as_deref(),
            avatar: None,
            pds_url: &state.pds_url,
            access_jwt: &sess.access_jwt,
            refresh_jwt: Some(&sess.refresh_jwt),
        },
    ).await?;

    Ok((StatusCode::CREATED, Json(LoginOutput {
        token,
        did: sess.did,
        handle: sess.handle,
        display_name: input.display_name,
        avatar: None,
    })))
}

/// Password login. Tries the home PDS first (new accounts), falls back to
/// the legacy platform_users table for pre-PDS signups. AT Protocol users on
/// a different PDS should use OAuth (`/oauth/login`) instead.
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> ApiResult<Json<LoginOutput>> {
    // 1. Home PDS password flow
    if !state.pds_url.is_empty() {
        let pds_handle = normalize_handle(&input.identifier, &state.pds_url);
        let client = reqwest::Client::new();
        match pds_xrpc::<PdsSession>(&client, &state.pds_url, "com.atproto.server.createSession",
            serde_json::json!({ "identifier": pds_handle, "password": input.password }),
        ).await {
            Ok(sess) => {
                let token = gen_session_token();
                let profile: Option<(Option<String>, Option<String>)> = sqlx::query_as(
                    "SELECT display_name, avatar_url FROM profiles WHERE did = $1",
                )
                .bind(&sess.did).fetch_optional(&state.pool).await?;
                let (display_name, avatar) = profile.unwrap_or((None, None));
                auth_service::create_session(
                    &state.pool,
                    &auth_service::CreateSessionInput {
                        token: &token, did: &sess.did, handle: &sess.handle,
                        display_name: display_name.as_deref(), avatar: avatar.as_deref(),
                        pds_url: &state.pds_url,
                        access_jwt: &sess.access_jwt,
                        refresh_jwt: Some(&sess.refresh_jwt),
                    },
                ).await?;
                return Ok(Json(LoginOutput {
                    token, did: sess.did, handle: sess.handle, display_name, avatar,
                }));
            }
            Err(AppError(fx_core::Error::BadRequest(_))) | Err(AppError(fx_core::Error::Unauthorized)) => {
                // Fall through to legacy path for old did:local accounts.
            }
            Err(e) => return Err(e),
        }
    }

    // 2. Legacy did:local platform user (pre-PDS accounts)
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
    Ok(StatusCode::NO_CONTENT)
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
