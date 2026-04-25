//! Inject NightBoat metadata into a typst source as a
//! `#metadata((...)) <nbt-article>` directive at the top of the file.
//!
//! Uses `typst-syntax::parse` to locate any existing `<nbt-article>`-labelled
//! metadata call so we never depend on regex against typst grammar (strings
//! containing `)`, multi-line dicts, escapes — all handled by the AST
//! walker). Replacing the existing directive is a precise byte-range edit.

use typst_syntax::{ast, parse, SyntaxKind, SyntaxNode};

use super::Metadata;
use super::{TYPST_ARTICLE_LABEL, TYPST_SERIES_LABEL};

/// Article-level convenience wrapper: merges into the `<nbt-article>`
/// directive at the top of a typst source.
pub fn merge(source: &str, incoming: &Metadata) -> String {
    merge_with_label(source, incoming, TYPST_ARTICLE_LABEL)
}

/// Series-level convenience wrapper: merges into the `<nbt-series>`
/// directive at the top of a typst source (typically the series's `main.typ`).
pub fn merge_series(source: &str, incoming: &Metadata) -> String {
    merge_with_label(source, incoming, TYPST_SERIES_LABEL)
}

/// Returns a typst source identical to `source` except that any existing
/// `#metadata((...)) <{label}>` directive has been replaced with one
/// reflecting `incoming` (or removed if `incoming` would emit nothing).
/// When no directive is present, the new one is prepended at the top.
pub fn merge_with_label(source: &str, incoming: &Metadata, label: &str) -> String {
    let new_dict = format_dict(incoming);
    let new_directive = if new_dict.is_empty() {
        String::new()
    } else {
        format!("#metadata(({new_dict})) <{label}>\n")
    };

    match find_directive_range(source, label) {
        Some(range) => {
            // Also swallow the immediately-following newline so we don't pile
            // up blank lines on every re-publish.
            let bytes = source.as_bytes();
            let mut end = range.end;
            if bytes.get(end) == Some(&b'\n') { end += 1; }
            let mut out = String::with_capacity(source.len() + new_directive.len());
            out.push_str(&source[..range.start]);
            out.push_str(&new_directive);
            out.push_str(&source[end..]);
            out
        }
        None => {
            if new_directive.is_empty() { return source.to_string(); }
            let leading_newline = if source.is_empty() || source.starts_with('\n') { "" } else { "\n" };
            format!("{new_directive}{leading_newline}{source}")
        }
    }
}

/// Find the byte range of an existing top-level `#metadata((...)) <{label}>`
/// span (including the call AND the label, but not any trailing newline).
fn find_directive_range(source: &str, label: &str) -> Option<std::ops::Range<usize>> {
    let root = parse(source);
    let kids: Vec<&SyntaxNode> = root.children().collect();
    let target_label = format!("<{label}>");
    let mut offset = 0usize;
    for (i, child) in kids.iter().enumerate() {
        let child_start = offset;
        let child_end = child_start + child.len();
        offset = child_end;
        // We're looking for a FuncCall `metadata(...)` whose next non-trivial
        // sibling is the matching label.
        let Some(call) = child.cast::<ast::FuncCall>() else { continue };
        let ast::Expr::Ident(ident) = call.callee() else { continue };
        if ident.as_str() != "metadata" { continue }
        let Some((label_end, label_kid)) = next_non_trivial(&kids, i, child_end) else {
            continue;
        };
        if label_kid.kind() != SyntaxKind::Label { continue }
        if label_kid.text().as_str() != target_label { continue }
        return Some(child_start..label_end);
    }
    None
}

/// Find the next sibling that is not whitespace/parbreak. Returns the byte
/// position immediately after that sibling, plus a reference to the sibling.
fn next_non_trivial<'a>(
    kids: &'a [&'a SyntaxNode],
    start_idx: usize,
    start_offset: usize,
) -> Option<(usize, &'a SyntaxNode)> {
    let mut offset = start_offset;
    for kid in kids.iter().skip(start_idx + 1) {
        let len = kid.len();
        match kid.kind() {
            SyntaxKind::Space | SyntaxKind::Parbreak => {
                offset += len;
                continue;
            }
            _ => return Some((offset + len, kid)),
        }
    }
    None
}

fn format_dict(meta: &Metadata) -> String {
    fn esc(s: &str) -> String {
        s.replace('\\', r"\\").replace('"', r#"\""#)
    }
    fn quoted(s: &str) -> String { format!("\"{}\"", esc(s)) }
    fn array(items: &[String]) -> String {
        // Typst arrays: empty `()`, multi `(a, b)`, but single MUST have a
        // trailing comma — `(x,)` — otherwise it parses as a parenthesised
        // expression, not a 1-element array.
        match items.len() {
            0 => "()".to_string(),
            1 => format!("({},)", quoted(&items[0])),
            _ => format!("({})",
                items.iter().map(|s| quoted(s)).collect::<Vec<_>>().join(", ")),
        }
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(t) = meta.title.as_deref().filter(|s| !s.is_empty())   { parts.push(format!("title: {}", quoted(t))); }
    if let Some(d) = meta.abstract_.as_deref().filter(|s| !s.is_empty()) { parts.push(format!("abstract: {}", quoted(d))); }
    if let Some(l) = meta.lang.as_deref().filter(|s| !s.is_empty())    { parts.push(format!("lang: {}", quoted(l))); }
    if let Some(c) = meta.category.as_deref().filter(|s| !s.is_empty()) { parts.push(format!("category: {}", quoted(c))); }
    if let Some(l) = meta.license.as_deref().filter(|s| !s.is_empty()) { parts.push(format!("license: {}", quoted(l))); }
    if let Some(c) = meta.cover.as_deref().filter(|s| !s.is_empty())   { parts.push(format!("cover: {}", quoted(c))); }
    if !meta.tags.is_empty()    { parts.push(format!("tags: {}", array(&meta.tags))); }
    if !meta.related.is_empty() { parts.push(format!("related: {}", array(&meta.related))); }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta_basic() -> Metadata {
        Metadata {
            title: Some("Hi".into()),
            lang: Some("zh".into()),
            license: Some("CC-BY-SA-4.0".into()),
            tags: vec!["calculus".into()],
            ..Default::default()
        }
    }

    #[test]
    fn prepends_when_absent() {
        let src = "= Heading\n\nbody\n";
        let out = merge(src, &meta_basic());
        assert!(out.starts_with("#metadata((title: \"Hi\""), "got: {out}");
        assert!(out.contains("<nbt-article>"));
        assert!(out.ends_with("= Heading\n\nbody\n"));
    }

    #[test]
    fn replaces_existing_directive() {
        let src = "#metadata((title: \"Old\")) <nbt-article>\n\n= Body\n";
        let out = merge(src, &meta_basic());
        assert!(out.contains("title: \"Hi\""));
        assert!(!out.contains("\"Old\""), "old directive should be gone: {out}");
        // single directive, not duplicated
        assert_eq!(out.matches("<nbt-article>").count(), 1);
        assert!(out.ends_with("= Body\n"));
    }

    #[test]
    fn empty_metadata_is_noop_when_no_directive() {
        let src = "= Body\n";
        let out = merge(src, &Metadata::default());
        assert_eq!(out, src);
    }

    #[test]
    fn empty_metadata_strips_existing_directive() {
        let src = "#metadata((title: \"Old\")) <nbt-article>\n\n= Body\n";
        let out = merge(src, &Metadata::default());
        assert!(!out.contains("<nbt-article>"), "got: {out:?}");
        assert!(!out.contains("\"Old\""), "got: {out:?}");
        assert!(out.contains("= Body"), "got: {out:?}");
    }

    #[test]
    fn handles_multiline_dict() {
        let src = "#metadata((\n  title: \"Old\",\n  lang: \"en\",\n)) <nbt-article>\n\n= Body\n";
        let out = merge(src, &meta_basic());
        assert!(out.contains("title: \"Hi\""));
        assert!(!out.contains("\"Old\""));
        assert!(!out.contains("\"en\""), "old lang should be gone: {out}");
    }

    #[test]
    fn handles_paren_inside_string() {
        // A `)` inside a string literal must NOT terminate the directive.
        let src = "#metadata((title: \"Hi (note))\")) <nbt-article>\n\n= Body\n";
        let out = merge(src, &meta_basic());
        assert_eq!(out.matches("<nbt-article>").count(), 1);
        assert!(out.contains("\"Hi\""));
    }

    #[test]
    fn escapes_quotes_in_values() {
        let m = Metadata { title: Some(r#"He said "hi""#.into()), ..Default::default() };
        let out = merge("= Body\n", &m);
        assert!(out.contains(r#""He said \"hi\"""#), "got: {out}");
    }

    #[test]
    fn series_label_independent_of_article_label() {
        // Article + series directives can coexist on the same file.
        let src = "#metadata((title: \"Article\")) <nbt-article>\n\n= Body\n";
        let series = Metadata { title: Some("Series".into()), ..Default::default() };
        let out = merge_series(src, &series);
        // article directive untouched
        assert!(out.contains("\"Article\""));
        assert!(out.contains("<nbt-article>"));
        // series directive added
        assert!(out.contains("\"Series\""));
        assert!(out.contains("<nbt-series>"));
    }
}
