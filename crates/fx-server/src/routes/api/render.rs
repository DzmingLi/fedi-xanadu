use axum::Json;
use fx_renderer::{SnippetRequest, SnippetResponse};

use crate::error::ApiResult;
use fx_core::Error;

/// POST /api/render/typst-snippet
/// Compiles a single Typst math expression and returns the HTML fragment.
/// Used by the Typst WYSIWYG editor to render math nodes in real time.
pub async fn render_typst_snippet(
    Json(body): Json<SnippetRequest>,
) -> ApiResult<Json<SnippetResponse>> {
    let formula = body.formula;
    let display = body.display;
    let html = tokio::task::spawn_blocking(move || fx_renderer::render_typst_math_snippet(&formula, display))
        .await
        .map_err(|e| Error::Internal(e.to_string()))?
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(Json(SnippetResponse { html }))
}

/// POST /api/render/latex-snippet
/// Converts a LaTeX math formula to MathML HTML.
pub async fn render_latex_snippet(
    Json(body): Json<SnippetRequest>,
) -> ApiResult<Json<SnippetResponse>> {
    let formula = body.formula;
    let display = body.display;
    let html = tokio::task::spawn_blocking(move || fx_renderer::render_latex_math_snippet(&formula, display))
        .await
        .map_err(|e| Error::Internal(e.to_string()))?
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(Json(SnippetResponse { html }))
}
