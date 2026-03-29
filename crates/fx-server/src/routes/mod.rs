pub mod api;

use axum::Router;
use axum::extract::Request;
use axum::response::Redirect;
use axum::routing::get;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use axum::response::{Html, IntoResponse, Json as AxumJson};
use utoipa::OpenApi;

use crate::config::Config;
use crate::openapi::ApiDoc;
use crate::state::AppState;

pub fn router(state: AppState, config: &Config) -> Router {
    // Serve the SPA: static files from frontend/dist, fallback to index.html
    let spa = ServeDir::new("frontend/dist")
        .not_found_service(ServeFile::new("frontend/dist/index.html"));

    // CORS: explicit origin whitelist, never permissive
    let cors = build_cors_layer(config);

    // Rate limiting: 200 requests per minute per real client IP
    // Uses SmartIpKeyExtractor to read X-Forwarded-For/X-Real-IP from Caddy,
    // instead of peer IP (which is always 127.0.0.1 behind the reverse proxy).
    let governor_conf = std::sync::Arc::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .key_extractor(tower_governor::key_extractor::SmartIpKeyExtractor)
            .per_second(4)
            .burst_size(200)
            .finish()
            .expect("invalid rate limiter config"),
    );
    let governor_limiter = tower_governor::GovernorLayer::new(governor_conf);

    Router::new()
        .route("/api/doc/openapi.json", get(openapi_json))
        .route("/api/doc", get(swagger_ui_html))
        .route("/metrics", get(crate::metrics::handler))
        .nest("/api/v1", api::routes())
        // Backwards-compatible redirect: /api/* → /api/v1/*
        .nest("/api", Router::new().fallback(api_v1_redirect))
        .fallback_service(spa)
        .layer(axum::middleware::from_fn(crate::metrics::track_requests))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(cors)
        .layer(governor_limiter)
        // Global body limit: 16 MB (image uploads are max 10 MB + overhead)
        .layer(RequestBodyLimitLayer::new(16 * 1024 * 1024))
        .with_state(state)
}

async fn openapi_json() -> impl IntoResponse {
    AxumJson(ApiDoc::openapi())
}

async fn swagger_ui_html() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html><head>
<title>Fedi-Xanadu API</title>
<link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head><body>
<div id="swagger-ui"></div>
<script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
<script>SwaggerUIBundle({ url: '/api/doc/openapi.json', dom_id: '#swagger-ui' });</script>
</body></html>"#)
}

/// Redirect old /api/* paths to /api/v1/* for backwards compatibility.
async fn api_v1_redirect(req: Request) -> Redirect {
    let path = req.uri().path();
    let rest = &path[4..]; // strip "/api"
    let query = req.uri().query().map(|q| format!("?{q}")).unwrap_or_default();
    Redirect::permanent(&format!("/api/v1{rest}{query}"))
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
        // No origins configured: only same-origin requests allowed
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
