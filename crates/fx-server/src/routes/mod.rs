pub mod api;

use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
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

    // Rate limiting: burst 600, refill 4/s per real client IP
    // Uses SmartIpKeyExtractor to read X-Forwarded-For/X-Real-IP from Caddy,
    // instead of peer IP (which is always 127.0.0.1 behind the reverse proxy).
    let governor_conf = std::sync::Arc::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .key_extractor(tower_governor::key_extractor::SmartIpKeyExtractor)
            .per_second(20)
            .burst_size(600)
            .finish()
            .expect("invalid rate limiter config"),
    );
    let governor_limiter = tower_governor::GovernorLayer::new(governor_conf);

    Router::new()
        .nest("/api", api::routes())
        .fallback_service(spa)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
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

    let origin_list = config.cors_origin_list();
    if origin_list.is_empty() {
        layer.allow_origin(AllowOrigin::exact(
            format!("http://{}:{}", config.host, config.port)
                .parse()
                .expect("invalid origin from config"),
        ))
    } else {
        let origins: Vec<_> = origin_list
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        layer.allow_origin(origins)
    }
}
