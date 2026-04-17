//! Description rendering helper.
//!
//! Article/series descriptions are authored in the same format as the content
//! (markdown or typst) and rendered to inline-only HTML for card previews.

use std::path::Path;
use std::sync::LazyLock;

use anyhow::Result;
use regex_lite::Regex;

/// Render description source to inline-only HTML: full render through
/// `fx_renderer`, then strip block-level wrappers so the result is safe to
/// embed in a card's `<p>` without breaking layout.
pub fn render_description_inline(format: &str, source: &str, repo_path: &Path) -> Result<String> {
    if source.trim().is_empty() {
        return Ok(String::new());
    }
    let config = fx_renderer::fx_render_config();
    let html = fx_renderer::render_to_html_with_config(format, source, repo_path, &config)?;
    Ok(inline_only(&html))
}

/// Strip block-level elements from rendered HTML, keeping inline content.
/// Block subtrees (h1-h6, lists, tables, pre, figure, blockquote) are dropped;
/// wrapper tags (p, div, section) are unwrapped; inline tags pass through.
fn inline_only(html: &str) -> String {
    static DROP_SELF: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"(?i)<(hr|br)\s*/?>",
    ).unwrap());
    static UNWRAP_BLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r"(?i)</?(p|div|section|article|header|footer|aside|main|nav)\b[^>]*>",
    ).unwrap());
    static WS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

    let mut out = html.to_string();
    for tag in ["pre", "script", "style", "figure", "table", "blockquote", "ul", "ol", "dl"] {
        out = drop_subtree_of(&out, tag);
    }
    for h in ["h1", "h2", "h3", "h4", "h5", "h6"] {
        out = drop_subtree_of(&out, h);
    }
    out = DROP_SELF.replace_all(&out, " ").to_string();
    out = UNWRAP_BLOCK.replace_all(&out, " ").to_string();
    WS.replace_all(out.trim(), " ").trim().to_string()
}

/// Remove `<tag ...>...</tag>` subtrees (non-greedy, case-insensitive).
/// regex-lite has no backreferences, so we take the tag name as a parameter
/// and compile a fresh regex per call (patterns are cheap).
fn drop_subtree_of(html: &str, tag: &str) -> String {
    let pattern = format!(r"(?is)<{tag}\b[^>]*>.*?</{tag}>");
    let re = Regex::new(&pattern).unwrap();
    re.replace_all(html, " ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_only_strips_headings_and_lists() {
        let html = "<h1>Title</h1><p>Intro <em>em</em>.</p><ul><li>a</li></ul><p>End.</p>";
        let got = inline_only(html);
        assert_eq!(got, "Intro <em>em</em>. End.");
    }

    #[test]
    fn empty_in_empty_out() {
        let p = std::path::Path::new(".");
        let got = render_description_inline("markdown", "", p).unwrap();
        assert_eq!(got, "");
    }
}
