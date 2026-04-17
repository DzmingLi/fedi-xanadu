//! Description rendering and auto-extraction helpers.
//!
//! Article/series descriptions are authored in the same format as the content
//! (markdown or typst) and rendered to inline-only HTML for card previews. When
//! `auto_description` is set, the source is derived from the content's rendered
//! plaintext so authors can leave it blank.

use std::path::Path;
use std::sync::LazyLock;

use anyhow::Result;
use regex_lite::Regex;

/// Max characters kept in an auto-extracted description excerpt.
pub const EXCERPT_MAX_CHARS: usize = 140;

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
    // Drop entire block subtrees (no regex backrefs in regex-lite, so iterate per tag).
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
fn drop_subtree_of(html: &str, tag: &str) -> String {
    let pattern = format!(r"(?is)<{tag}\b[^>]*>.*?</{tag}>");
    let re = Regex::new(&pattern).unwrap();
    re.replace_all(html, " ").to_string()
}

/// Extract plaintext excerpt from rendered content HTML for auto-description.
/// Strips HTML tags (skipping `<pre>`, `<script>`, `<style>`, math displays,
/// `<figure>`), collapses whitespace, truncates to `EXCERPT_MAX_CHARS` at a
/// word/CJK boundary.
pub fn extract_excerpt_from_html(html: &str) -> String {
    static KATEX_SPAN: LazyLock<Regex> = LazyLock::new(|| Regex::new(
        r#"(?is)<span[^>]*class="[^"]*\bkatex[a-z-]*\b[^"]*"[^>]*>.*?</span>"#,
    ).unwrap());
    static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").unwrap());
    static WS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

    let mut text = html.to_string();
    for tag in ["pre", "script", "style", "figure", "code", "math"] {
        text = drop_subtree_of(&text, tag);
    }
    text = KATEX_SPAN.replace_all(&text, " ").to_string();
    text = TAGS.replace_all(&text, " ").to_string();
    // Decode a minimal set of HTML entities.
    text = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    text = WS.replace_all(text.trim(), " ").trim().to_string();
    truncate_at_boundary(&text, EXCERPT_MAX_CHARS)
}

/// Truncate to at most `max_chars` unicode scalar values, preferring to break
/// after a CJK punctuation or at whitespace within the last 20 chars.
/// Appends `…` when truncation occurs.
fn truncate_at_boundary(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars { return s.to_string(); }
    let mut cut = max_chars;
    let lookback = max_chars.saturating_sub(20);
    for i in (lookback..max_chars).rev() {
        let c = chars[i];
        if c.is_whitespace() || is_cjk_break(c) {
            cut = i + 1;
            break;
        }
    }
    let mut out: String = chars[..cut].iter().collect();
    out = out.trim_end().to_string();
    out.push('…');
    out
}

fn is_cjk_break(c: char) -> bool {
    matches!(c, '。' | '，' | '；' | '：' | '！' | '？' | '、')
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
    fn excerpt_drops_code_and_katex() {
        let html = r#"<p>Hello <code>x</code> <span class="katex">formula</span> world.</p>"#;
        let got = extract_excerpt_from_html(html);
        assert_eq!(got, "Hello world.");
    }

    #[test]
    fn excerpt_truncates_at_cjk_boundary() {
        let long = "这是一段很长的中文文本，".repeat(20);
        let got = extract_excerpt_from_html(&long);
        assert!(got.chars().count() <= EXCERPT_MAX_CHARS + 1);
        assert!(got.ends_with('…'));
    }

    #[test]
    fn empty_in_empty_out() {
        let p = std::path::Path::new(".");
        let got = render_description_inline("markdown", "", p).unwrap();
        assert_eq!(got, "");
    }
}
