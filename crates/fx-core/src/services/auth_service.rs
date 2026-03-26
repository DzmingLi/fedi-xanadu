use sqlx::PgPool;

use crate::Result;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct SessionInfo {
    pub did: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PdsSession {
    pub did: String,
    pub pds_url: String,
    pub access_jwt: String,
}

pub struct CreateSessionInput<'a> {
    pub token: &'a str,
    pub did: &'a str,
    pub handle: &'a str,
    pub display_name: Option<&'a str>,
    pub avatar: Option<&'a str>,
    pub pds_url: &'a str,
    pub access_jwt: &'a str,
    pub refresh_jwt: Option<&'a str>,
}

/// Insert a new session row (or replace on token conflict).
pub async fn create_session(pool: &PgPool, input: &CreateSessionInput<'_>) -> Result<()> {
    sqlx::query(
        "INSERT INTO sessions (token, did, handle, display_name, avatar_url, pds_url, access_jwt, refresh_jwt)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (token) DO UPDATE SET
           did = EXCLUDED.did, handle = EXCLUDED.handle,
           display_name = EXCLUDED.display_name, avatar_url = EXCLUDED.avatar_url,
           pds_url = EXCLUDED.pds_url, access_jwt = EXCLUDED.access_jwt,
           refresh_jwt = EXCLUDED.refresh_jwt",
    )
    .bind(input.token)
    .bind(input.did)
    .bind(input.handle)
    .bind(input.display_name)
    .bind(input.avatar)
    .bind(input.pds_url)
    .bind(input.access_jwt)
    .bind(input.refresh_jwt)
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert or update a profile row keyed by DID.
pub async fn upsert_profile(
    pool: &PgPool,
    did: &str,
    handle: &str,
    display_name: Option<&str>,
    avatar: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO profiles (did, handle, display_name, avatar_url) VALUES ($1, $2, $3, $4)
         ON CONFLICT(did) DO UPDATE SET handle = EXCLUDED.handle, display_name = EXCLUDED.display_name,
         avatar_url = EXCLUDED.avatar_url, updated_at = NOW()",
    )
    .bind(did)
    .bind(handle)
    .bind(display_name)
    .bind(avatar)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete a session by its bearer token.
pub async fn delete_session(pool: &PgPool, token: &str) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token = $1")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

/// Look up session info (did, handle, display_name, avatar) by bearer token.
/// Returns `None` if the token is missing or expired.
pub async fn get_session_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<SessionInfo>> {
    let row = sqlx::query_as::<_, SessionInfo>(
        "SELECT did, handle, display_name, avatar_url FROM sessions WHERE token = $1 AND expires_at > NOW()",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Look up PDS connection details (did, pds_url, access_jwt) by bearer token.
/// Returns `None` if the token is missing or expired.
pub async fn get_session_for_pds(
    pool: &PgPool,
    token: &str,
) -> Result<Option<PdsSession>> {
    let row = sqlx::query_as::<_, PdsSession>(
        "SELECT did, pds_url, access_jwt FROM sessions WHERE token = $1 AND expires_at > NOW()",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Delete sessions that have expired. Call periodically (e.g. once per hour).
pub async fn cleanup_expired_sessions(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Retrieve just the DID for an active session token.
/// Returns `None` if the token is missing or expired.
pub async fn get_did_by_token(pool: &PgPool, token: &str) -> Result<Option<String>> {
    let did = sqlx::query_scalar::<_, String>(
        "SELECT did FROM sessions WHERE token = $1 AND expires_at > NOW()",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;
    Ok(did)
}
