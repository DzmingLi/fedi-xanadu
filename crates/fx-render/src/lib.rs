pub mod convert;
pub mod markdown_render;
pub mod typst_render;

use std::path::Path;

pub use convert::convert_format;
pub use markdown_render::render_markdown_to_html;
pub use typst_render::{render_typst_to_html, render_typst_to_html_with_images, render_series_to_html};

/// Map a content format identifier to its canonical file extension.
pub fn format_extension(format: &str) -> &'static str {
    match format {
        "markdown" => "md",
        "html" => "html",
        _ => "typ",
    }
}

/// Render source content to HTML based on format.
///
/// `repo_path` is used by Typst to resolve images; other formats ignore it.
pub fn render_to_html(format: &str, source: &str, repo_path: &Path) -> anyhow::Result<String> {
    match format {
        "markdown" => render_markdown_to_html(source),
        "html" => Ok(source.to_string()),
        _ => render_typst_to_html_with_images(source, repo_path),
    }
}
