use axum::extract::MatchedPath;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::LazyLock;
use std::time::Instant;

static PROMETHEUS_HANDLE: LazyLock<PrometheusHandle> = LazyLock::new(|| {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder")
});

/// Initialize the metrics recorder. Must be called before any metrics are recorded.
pub fn init() {
    LazyLock::force(&PROMETHEUS_HANDLE);
}

/// Handler for GET /metrics — returns Prometheus text format.
pub async fn handler() -> impl IntoResponse {
    PROMETHEUS_HANDLE.render()
}

/// Middleware that records request count and latency per route.
pub async fn track_requests(
    matched_path: Option<MatchedPath>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = matched_path
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());

    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    metrics::counter!("http_requests_total", "method" => method.clone(), "path" => path.clone(), "status" => status)
        .increment(1);
    metrics::histogram!("http_request_duration_seconds", "method" => method, "path" => path)
        .record(latency);

    response
}
