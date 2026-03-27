mod config;
mod error;
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
    let app = routes::router(state.clone(), &config);

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
