pub mod api;

use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::state::AppState;

pub fn router(state: AppState, config: &Config) -> Router {
    // Serve the SPA: static files from frontend/dist, fallback to index.html
    let spa = ServeDir::new("frontend/dist")
        .not_found_service(ServeFile::new("frontend/dist/index.html"));

    // CORS: explicit origin whitelist, never permissive
    let cors = build_cors_layer(config);

    // Rate limiting: 60 requests per minute per IP
    let governor_conf = std::sync::Arc::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(60)
            .finish()
            .expect("invalid rate limiter config"),
    );
    let governor_limiter = tower_governor::GovernorLayer::new(governor_conf);

    Router::new()
        .nest("/api", api::routes())
        .fallback_service(spa)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(governor_limiter)
        // Global body limit: 16 MB (image uploads are max 10 MB + overhead)
        .layer(RequestBodyLimitLayer::new(16 * 1024 * 1024))
        .with_state(state)
}

fn build_cors_layer(config: &Config) -> CorsLayer {
    use axum::http::{HeaderName, Method};

    let methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ];

    let headers = vec![
        HeaderName::from_static("authorization"),
        HeaderName::from_static("content-type"),
    ];

    let layer = CorsLayer::new()
        .allow_methods(methods)
        .allow_headers(headers);

    if config.cors_origins.is_empty() {
        // No origins configured: only same-origin requests allowed
        layer.allow_origin(AllowOrigin::exact(
            format!("http://{}:{}", config.host, config.port)
                .parse()
                .expect("invalid origin from config"),
        ))
    } else {
        let origins: Vec<_> = config
            .cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        layer.allow_origin(origins)
    }
}
