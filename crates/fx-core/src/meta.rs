//! Series and article metadata, authored as YAML.
//!
//! The pijul repo is the source of truth: every article/series is defined
//! by its content files plus YAML metadata. The database is an indexed
//! cache that can be rebuilt from pijul contents.
//!
//! # Where metadata lives
//!
//! **Series repo** — `meta.yaml` at the repo root:
//!
//! ```yaml
//! title: Static Program Analysis
//! description: Companion textbook to NJU's software analysis course
//! lang: zh
//! category: lecture
//! topics: [static-analysis, compiler]
//! split_level: 2
//!
//! # Only meaningful for markdown series. Typst series derive chapters
//! # from main.typ compilation + split_level.
//! chapters:
//!   - README.md
//!   - ch0/ch0.md
//! ```
//!
//! **Markdown chapter** — YAML frontmatter at the top of each `.md` file:
//!
//! ```markdown
//! ---
//! title: Preface
//! teaches: [intro-tag]
//! prereqs:
//!   - tag: cs-basics
//!   - tag: lattice-theory
//!     type: recommended
//! ---
//!
//! # Preface
//! ...
//! ```
//!
//! **Standalone markdown article** — same frontmatter pattern in `content.md`.
//! Standalone typst/html articles keep their metadata in the database.

use serde::{Deserialize, Serialize};

pub const SERIES_META_FILENAME: &str = "meta.yaml";

// ── SeriesMeta ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeriesMeta {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    /// Splits the compiled series into pages at this heading level (1-6).
    /// Used by typst series; for markdown series, each file is already a
    /// page and `split_level` only controls per-file subsection splitting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_level: Option<u32>,
    /// Ordered list of chapter file paths relative to the repo root.
    /// Only meaningful for markdown series — typst series compile main.typ
    /// and derive chapters from heading splits at runtime.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<String>,
    /// Relative path (within the repo) to the cover image. Authoritative —
    /// surviving fork/clone — so a cloned series keeps its cover selection
    /// without needing the original database.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
}

/// Read meta.yaml, set (or clear) the cover field, write it back.
pub fn set_series_meta_cover(dir: &std::path::Path, cover: Option<String>) -> std::io::Result<()> {
    let mut meta = read_series_meta(dir).unwrap_or_default();
    meta.cover = cover;
    write_series_meta(dir, &meta)
}

pub fn read_series_meta(dir: &std::path::Path) -> Option<SeriesMeta> {
    let path = dir.join(SERIES_META_FILENAME);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_yml::from_str(&data).ok()
}

pub fn write_series_meta(dir: &std::path::Path, meta: &SeriesMeta) -> std::io::Result<()> {
    let yaml = serde_yml::to_string(meta)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(dir.join(SERIES_META_FILENAME), yaml)
}

// ── Per-chapter frontmatter ─────────────────────────────────────────────

/// Frontmatter parsed from the top of a markdown file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Relative path (within the article's pijul repo) to a cover image.
    /// Authoritative — surviving fork/clone — so other consumers keep the
    /// cover selection without needing the origin database.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub teaches: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prereqs: Vec<PrereqEntry>,
    /// Concepts the article touches without teaching — application
    /// domains, historical context, tangential references. Feeds the
    /// tag page's "related" list but not skill inference.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,
}

/// Rewrite a markdown source so its frontmatter `cover` field matches `cover`.
/// If the source has no frontmatter, one is synthesised; if it had one, only
/// the cover field is touched (other keys are preserved by round-tripping
/// through `Frontmatter` — any key we don't model is LOST, so this is meant
/// for fields the app controls).
pub fn rewrite_markdown_cover(source: &str, cover: Option<String>) -> String {
    let (mut fm, body) = split_frontmatter(source);
    fm.cover = cover;
    let yaml = serde_yml::to_string(&fm).unwrap_or_default();
    let trimmed = yaml.trim_end();
    // Empty frontmatter (everything optional/empty) → emit no block.
    if trimmed.is_empty() || trimmed == "{}" {
        return body.to_string();
    }
    format!("---\n{trimmed}\n---\n{body}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrereqEntry {
    pub tag: String,
    /// `None` → required (default). `Some("recommended")` for soft prereq.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub prereq_type: Option<String>,
}

impl PrereqEntry {
    /// Returns the prereq strength, defaulting to "required" when unspecified.
    pub fn kind(&self) -> &str {
        self.prereq_type.as_deref().unwrap_or("required")
    }
}

/// Split a markdown source into (frontmatter, body). Returns
/// `(Frontmatter::default(), source)` when no frontmatter is present.
///
/// Recognises the standard pattern:
/// ```text
/// ---
/// key: value
/// ---
/// # Actual heading
/// body...
/// ```
pub fn split_frontmatter(source: &str) -> (Frontmatter, &str) {
    let Some(rest) = source.strip_prefix("---") else {
        return (Frontmatter::default(), source);
    };
    // A proper opener is `---` followed by a newline (Windows or Unix).
    let rest = match rest.strip_prefix('\n').or_else(|| rest.strip_prefix("\r\n")) {
        Some(r) => r,
        None => return (Frontmatter::default(), source),
    };

    // Find the closing delimiter at the start of a line.
    let Some((yaml, body)) = find_frontmatter_close(rest) else {
        return (Frontmatter::default(), source);
    };
    let fm = serde_yml::from_str::<Frontmatter>(yaml).unwrap_or_default();
    (fm, body)
}

fn find_frontmatter_close(rest: &str) -> Option<(&str, &str)> {
    let mut search_from = 0;
    while let Some(idx) = rest[search_from..].find("---") {
        let abs = search_from + idx;
        // Must be at start of line
        let at_line_start = abs == 0 || rest.as_bytes()[abs - 1] == b'\n';
        if !at_line_start {
            search_from = abs + 3;
            continue;
        }
        // Must be followed by newline or EOF
        let after = &rest[abs + 3..];
        let body_start = if let Some(r) = after.strip_prefix('\n') { r }
            else if let Some(r) = after.strip_prefix("\r\n") { r }
            else if after.is_empty() { after }
            else { search_from = abs + 3; continue; };
        let yaml = rest[..abs].trim_end_matches(['\n', '\r']);
        return Some((yaml, body_start));
    }
    None
}

// ── Conversions ─────────────────────────────────────────────────────────

/// Default relative path inside a series pijul repo for a chapter.
/// Used when the caller doesn't specify one (regular publish flow).
/// Batch-publish overrides with explicit paths.
pub fn default_chapter_path(chapter_id: &str, src_ext: &str) -> String {
    format!("chapters/{chapter_id}.{src_ext}")
}

// ── Title extraction ────────────────────────────────────────────────────

/// Extract a title from the first heading in a content file, based on format.
/// For markdown: skips YAML frontmatter, then finds the first `# ` line.
pub fn extract_first_heading(source: &str, format: &str) -> Option<String> {
    match format.trim() {
        "markdown" | "md" => {
            let (_, body) = split_frontmatter(source);
            body.lines()
                .find_map(|line| line.trim_end().strip_prefix("# ").map(|s| s.trim().to_string()))
        }
        "typst" | "typ" => source
            .lines()
            .find_map(|line| line.trim_start().strip_prefix("= ").map(|s| s.trim().to_string())),
        "html" => extract_html_heading(source),
        _ => None,
    }
}

fn extract_html_heading(source: &str) -> Option<String> {
    let lower = source.to_lowercase();
    let start = lower.find("<h1")?;
    let after_open_bracket = source[start..].find('>')? + start + 1;
    let end = source[after_open_bracket..].to_lowercase().find("</h1>")? + after_open_bracket;
    let inner = source[after_open_bracket..end].trim();
    if inner.is_empty() { None } else { Some(strip_html_tags(inner)) }
}

fn strip_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_series_meta() {
        let meta = SeriesMeta {
            title: "Demo".into(),
            description: Some("一句话说明".into()),
            lang: Some("zh".into()),
            category: Some("lecture".into()),
            topics: vec!["a".into(), "b".into()],
            split_level: Some(2),
            chapters: vec!["README.md".into(), "ch0/ch0.md".into()],
            ..Default::default()
        };
        let yaml = serde_yml::to_string(&meta).unwrap();
        let back: SeriesMeta = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(back.title, meta.title);
        assert_eq!(back.description, meta.description);
        assert_eq!(back.chapters, meta.chapters);
        assert_eq!(back.split_level, meta.split_level);
    }

    #[test]
    fn frontmatter_basic() {
        let src = "---\ntitle: Preface\nteaches:\n  - x\n  - y\nprereqs:\n  - tag: a\n  - tag: b\n    type: recommended\n---\n# Preface\n\nbody\n";
        let (fm, body) = split_frontmatter(src);
        assert_eq!(fm.title.as_deref(), Some("Preface"));
        assert_eq!(fm.teaches, vec!["x".to_string(), "y".to_string()]);
        assert_eq!(fm.prereqs.len(), 2);
        assert_eq!(fm.prereqs[0].kind(), "required");
        assert_eq!(fm.prereqs[1].kind(), "recommended");
        assert!(body.starts_with("# Preface"));
    }

    #[test]
    fn frontmatter_missing() {
        let src = "# Just content\nnothing above\n";
        let (fm, body) = split_frontmatter(src);
        assert!(fm.title.is_none());
        assert_eq!(body, src);
    }

    #[test]
    fn frontmatter_trailing_content_only() {
        // `---` inside body shouldn't trigger false match when no opening.
        let src = "# Heading\n\n---\nthree dashes mid-body\n";
        let (fm, body) = split_frontmatter(src);
        assert!(fm.title.is_none());
        assert_eq!(body, src);
    }

    #[test]
    fn extract_md_heading_skips_frontmatter() {
        let src = "---\ntitle: foo\n---\n# After frontmatter\n";
        assert_eq!(extract_first_heading(src, "md").as_deref(), Some("After frontmatter"));
    }

    #[test]
    fn extract_typst_heading_top() {
        assert_eq!(extract_first_heading("= Top\n== Sub", "typst").as_deref(), Some("Top"));
    }

    #[test]
    fn extract_html_heading_simple() {
        assert_eq!(extract_first_heading("<h1>Header</h1>", "html").as_deref(), Some("Header"));
        assert_eq!(
            extract_first_heading("<h1 class=\"x\">With <em>tags</em></h1>", "html").as_deref(),
            Some("With tags"),
        );
    }
}
