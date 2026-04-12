use axum::Json;
use fx_renderer::typst_render::render_typst_to_html;
use serde::{Deserialize, Serialize};

use crate::error::ApiResult;
use fx_core::Error;

#[derive(Deserialize)]
pub struct SnippetRequest {
    pub formula: String,
    /// true = display math (centred block), false = inline
    pub display: bool,
}

#[derive(Serialize)]
pub struct SnippetResponse {
    pub html: String,
}

/// POST /api/render/typst-snippet
/// Compiles a single Typst math expression and returns the HTML fragment.
/// Used by the Typst WYSIWYG editor to render math nodes in real time.
pub async fn render_typst_snippet(
    Json(body): Json<SnippetRequest>,
) -> ApiResult<Json<SnippetResponse>> {
    let formula = body.formula.trim().to_string();
    if formula.is_empty() {
        return Ok(Json(SnippetResponse { html: String::new() }));
    }

    // Wrap in a minimal Typst document so the renderer can compile it.
    let source = if body.display {
        format!("$ {formula} $\n")
    } else {
        format!("${formula}$\n")
    };

    let html = tokio::task::spawn_blocking(move || render_typst_to_html(&source))
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
    let formula = body.formula.trim().to_string();
    if formula.is_empty() {
        return Ok(Json(SnippetResponse { html: String::new() }));
    }

    let display = body.display;
    let html = tokio::task::spawn_blocking(move || fx_renderer::render_latex_to_mathml(&formula, display))
        .await
        .map_err(|e| Error::Internal(e.to_string()))?
        .map_err(|e| Error::Internal(e.to_string()))?;

    Ok(Json(SnippetResponse { html }))
}
