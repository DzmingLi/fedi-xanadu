use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::{SaltString, rand_core::OsRng}};
use sqlx::PgPool;

use crate::Result;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlatformUser {
    pub did: String,
    pub handle: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct PlatformUserInfo {
    pub did: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Build a deterministic DID for a platform-local user.
pub fn local_did(handle: &str) -> String {
    format!("did:local:{handle}")
}

/// Hash a password with argon2.
pub fn hash_password(password: &str) -> std::result::Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verify a password against an argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> std::result::Result<bool, argon2::password_hash::Error> {
    let parsed = PasswordHash::new(hash)?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}

/// Create a new platform user. Returns the DID.
pub async fn create_platform_user(
    pool: &PgPool,
    handle: &str,
    display_name: Option<&str>,
    password: &str,
) -> Result<String> {
    let did = local_did(handle);
    let pw_hash = hash_password(password)
        .map_err(|e| crate::Error::Internal(format!("password hash error: {e}")))?;

    sqlx::query(
        "INSERT INTO platform_users (did, handle, display_name, password_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(&did)
    .bind(handle)
    .bind(display_name)
    .bind(&pw_hash)
    .execute(pool)
    .await?;

    // Also create a profile row so the user shows up everywhere
    sqlx::query(
        "INSERT INTO profiles (did, handle, display_name) VALUES ($1, $2, $3)
         ON CONFLICT(did) DO UPDATE SET handle = EXCLUDED.handle, display_name = EXCLUDED.display_name, updated_at = NOW()",
    )
    .bind(&did)
    .bind(handle)
    .bind(display_name)
    .execute(pool)
    .await?;

    Ok(did)
}

/// Look up a platform user by handle for login.
pub async fn get_by_handle(pool: &PgPool, handle: &str) -> Result<Option<PlatformUser>> {
    let row = sqlx::query_as::<_, PlatformUser>(
        "SELECT did, handle, password_hash FROM platform_users WHERE handle = $1",
    )
    .bind(handle)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// List all platform users (admin).
pub async fn list_platform_users(pool: &PgPool) -> Result<Vec<PlatformUserInfo>> {
    let rows = sqlx::query_as::<_, PlatformUserInfo>(
        "SELECT did, handle, display_name, created_at FROM platform_users ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
