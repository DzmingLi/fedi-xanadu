pub mod auth;
mod config;
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
    let oauth_config = atproto_auth::OAuthConfig::new_dev(&public_url, &config.instance_name)?;
    tracing::info!("OAuth client_id: {}", oauth_config.client_id());

    let oauth_request_store: std::sync::Arc<dyn atproto_oauth::storage::OAuthRequestStorage> =
        std::sync::Arc::new(atproto_auth::MemoryRequestStore::new());

    let oauth_state = atproto_auth::OAuthState {
        config: oauth_config,
        request_store: oauth_request_store,
        session_store: state.session_store.clone(),
        http_client: reqwest::Client::new(),
    };

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
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
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
