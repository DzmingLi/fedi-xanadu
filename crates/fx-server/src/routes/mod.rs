pub mod api;

use axum::{Router, extract::State, response::IntoResponse, http::header};
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
        .route("/sitemap.xml", axum::routing::get(sitemap_handler))
        .route("/feed/{did}.xml", axum::routing::get(rss_feed_handler))
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

async fn sitemap_handler(State(state): State<AppState>) -> impl IntoResponse {
    let articles: Vec<(String,)> = sqlx::query_as(
        "SELECT at_uri FROM articles WHERE removed_at IS NULL AND visibility = 'public' ORDER BY created_at DESC"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let series: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM series WHERE is_published = TRUE ORDER BY created_at DESC"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let base = std::env::var("FX_PUBLIC_URL").unwrap_or_else(|_| "https://fedi-xanadu.dzming.li".into());
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    xml.push_str(&format!("  <url><loc>{base}/</loc></url>\n"));

    for (uri,) in &articles {
        xml.push_str(&format!("  <url><loc>{base}/article?uri={}</loc></url>\n", urlencoding::encode(uri)));
    }
    for (id,) in &series {
        xml.push_str(&format!("  <url><loc>{base}/series?id={}</loc></url>\n", urlencoding::encode(id)));
    }

    xml.push_str("</urlset>");
    ([(header::CONTENT_TYPE, "application/xml")], xml)
}

async fn rss_feed_handler(
    State(state): State<AppState>,
    axum::extract::Path(did): axum::extract::Path<String>,
) -> impl IntoResponse {
    let base = std::env::var("FX_PUBLIC_URL").unwrap_or_else(|_| "https://fedi-xanadu.dzming.li".into());

    let handle: Option<String> = sqlx::query_scalar("SELECT handle FROM platform_users WHERE did = $1")
        .bind(&did).fetch_optional(&state.pool).await.ok().flatten();
    let author = handle.as_deref().unwrap_or(&did);

    let articles: Vec<(String, String, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT at_uri, title, description, created_at FROM articles \
         WHERE did = $1 AND removed_at IS NULL AND visibility = 'public' \
         ORDER BY created_at DESC LIMIT 50"
    ).bind(&did).fetch_all(&state.pool).await.unwrap_or_default();

    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
  <title>{author} - Fedi-Xanadu</title>
  <link>{base}/profile?did={did}</link>
  <description>Articles by {author} on Fedi-Xanadu</description>
  <atom:link href="{base}/feed/{did}.xml" rel="self" type="application/rss+xml"/>
"#
    );

    for (uri, title, desc, created) in &articles {
        let link = format!("{base}/article?uri={}", urlencoding::encode(uri));
        let pub_date = created.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let title_escaped = title.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let desc_escaped = desc.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        xml.push_str(&format!(
            "  <item>\n    <title>{title_escaped}</title>\n    <link>{link}</link>\n    <description>{desc_escaped}</description>\n    <pubDate>{pub_date}</pubDate>\n    <guid>{link}</guid>\n  </item>\n"
        ));
    }

    xml.push_str("</channel>\n</rss>");
    ([(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")], xml)
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
