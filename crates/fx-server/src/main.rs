pub mod auth;
pub mod avatar_cache;
mod config;
mod summary;
mod error;
mod prerender;
mod routes;
mod state;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let is_production = std::env::var("FX_ENV").as_deref() == Ok("production");
    if is_production {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .init();
    }

    let config = config::Config::load()?;
    let state = state::AppState::new(&config).await?;

    // OAuth setup
    let public_url = if config.public_url.is_empty() {
        format!("http://{}:{}", config.host, config.port)
    } else {
        config.public_url.clone()
    };
    // Signing key must persist across restarts — otherwise every redeploy
    // invalidates in-flight OAuth flows (PDS signs client_assertion with
    // one key, we verify with a different key → invalid_client).
    let oauth_config = load_or_create_oauth_config(
        &public_url,
        &config.instance_name,
        &std::path::PathBuf::from(&config.blob_cache_path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("oauth-signing-key"),
    )?;
    tracing::info!("OAuth client_id: {}", oauth_config.client_id());

    let oauth_request_store: std::sync::Arc<dyn atproto_oauth::storage::OAuthRequestStorage> =
        std::sync::Arc::new(atproto_auth::MemoryRequestStore::new());

    let oauth_state = atproto_auth::OAuthState::new(
        oauth_config,
        oauth_request_store,
        state.session_store.clone(),
    );

    let seo_template = std::fs::read_to_string("frontend/dist/index.html")
        .unwrap_or_else(|_| {
            tracing::warn!("frontend/dist/index.html not found, SEO meta injection disabled");
            String::new()
        });

    let seo_state = prerender::SeoState {
        pool: state.pool.clone(),
        template: std::sync::Arc::new(seo_template),
        public_url: public_url.clone(),
    };

    let app = routes::router(state.clone(), &config)
        .nest("/oauth", atproto_auth::oauth_router(oauth_state))
        .layer(axum::middleware::from_fn_with_state(
            seo_state,
            prerender::seo_middleware,
        ));

    // Background task: clean up expired sessions every hour
    let cleanup_pool = state.pool.clone();
    let sync_at_client = state.at_client.clone();
    let sync_data_dir = state.data_dir.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        let mut tick: u64 = 0;
        loop {
            interval.tick().await;
            match fx_core::services::auth_service::cleanup_expired_sessions(&cleanup_pool).await {
                Ok(n) if n > 0 => tracing::info!("cleaned up {n} expired sessions"),
                Err(e) => tracing::warn!("session cleanup failed: {e}"),
                _ => {}
            }
            // Recalculate all user reputations (safety net for consistency)
            match fx_core::services::reputation_service::recalc_all(&cleanup_pool).await {
                Ok(n) if n > 0 => tracing::debug!("recalculated reputation for {n} users"),
                Err(e) => tracing::warn!("reputation recalc failed: {e}"),
                _ => {}
            }
            // Hard-delete articles removed over 30 days ago
            match fx_core::services::article_service::cleanup_expired_removals(&cleanup_pool).await {
                Ok(n) if n > 0 => tracing::info!("hard-deleted {n} expired removed articles"),
                Err(e) => tracing::warn!("article cleanup failed: {e}"),
                _ => {}
            }

            // Daily: re-sync Bluesky profiles (handle / display_name / avatar).
            // The on-demand sync in get_profile already caches for active
            // users; this catches stale rows no one has opened recently.
            if tick % 24 == 0 {
                let n = resync_bsky_profiles(&cleanup_pool, &sync_at_client, &sync_data_dir).await;
                if n > 0 {
                    tracing::info!("re-synced {n} Bluesky profiles");
                }
            }
            tick = tick.wrapping_add(1);
        }
    });

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl+c");
    tracing::info!("shutting down");
}

fn load_or_create_oauth_config(
    public_url: &str,
    client_name: &str,
    key_path: &std::path::Path,
) -> Result<atproto_auth::OAuthConfig> {
    use atproto_identity::key::{KeyType, generate_key, identify_key};

    if key_path.exists() {
        let stored = std::fs::read_to_string(key_path)?;
        let key = identify_key(stored.trim())
            .map_err(|e| anyhow::anyhow!("parse stored oauth key: {e}"))?;
        tracing::info!("OAuth signing key: loaded from {}", key_path.display());
        return Ok(atproto_auth::OAuthConfig::new(public_url, client_name, key));
    }

    let key = generate_key(KeyType::P256Private)
        .map_err(|e| anyhow::anyhow!("generate oauth key: {e}"))?;
    if let Some(parent) = key_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(key_path, format!("{}", key))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(key_path, std::fs::Permissions::from_mode(0o600))?;
    }
    tracing::info!("OAuth signing key: generated and saved to {}", key_path.display());
    Ok(atproto_auth::OAuthConfig::new(public_url, client_name, key))
}

/// Re-fetch every AT Proto user's profile from the Bluesky public API and
/// refresh our cache (handle, display_name, avatar_url). Avatars are
/// downloaded to disk so the browser never needs to hit the Bluesky CDN.
async fn resync_bsky_profiles(
    pool: &sqlx::PgPool,
    at_client: &fx_atproto::client::AtClient,
    data_dir: &std::path::Path,
) -> usize {
    let dids: Vec<String> = match sqlx::query_scalar::<_, String>(
        "SELECT did FROM profiles WHERE did LIKE 'did:plc:%' OR did LIKE 'did:web:%'",
    )
    .fetch_all(pool)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("resync_bsky_profiles: fetch dids failed: {e}");
            return 0;
        }
    };

    let mut count = 0;
    for did in dids {
        let Ok(bsky) = at_client.get_public_profile(&did).await else { continue };

        // Bluesky's public AppView returns handle="handle.invalid" for DIDs
        // whose data it can't verify — notably accounts on PDSes it doesn't
        // federate with (e.g. our pds.nightbo.at). Skip the handle+display
        // overwrite in that case so we don't clobber the good DB value with
        // AppView's ignorance.
        if bsky.handle == "handle.invalid" {
            tracing::debug!("resync: skipping {did} (AppView returned handle.invalid)");
            continue;
        }

        let cached = match bsky.avatar.as_deref() {
            Some(remote) => avatar_cache::cache_remote_avatar(data_dir, &did, remote).await,
            None => None,
        };
        let av = cached.or_else(|| bsky.avatar.clone());
        if let Err(e) = sqlx::query(
            "UPDATE profiles SET handle = $1, display_name = $2, \
             avatar_url = $3, banner_url = COALESCE($4, banner_url) WHERE did = $5",
        )
        .bind(&bsky.handle)
        .bind(&bsky.display_name)
        .bind(&av)
        .bind(&bsky.banner)
        .bind(&did)
        .execute(pool)
        .await
        {
            tracing::debug!("resync: update {did} failed: {e}");
            continue;
        }
        count += 1;
    }
    count
}
