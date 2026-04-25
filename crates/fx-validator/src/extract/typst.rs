//! Metadata extraction for Typst article files.
//!
//! Contract: the `main.typ` file must contain exactly one labelled metadata
//! call of the form
//!
//! ```typst
//! #metadata((
//!   lang: "zh-CN",
//!   translation-of: "../source/main.typ",
//!   title: "Title",
//!   tags: ("physics", "qm"),
//! )) <nightboat-translation>
//! ```
//!
//! Rules:
//! - The metadata argument must be a dict literal with string, bool, or
//!   string-array values. No expressions, no references.
//! - The label `<nightboat-translation>` must appear as the next non-whitespace
//!   token after the closing paren of the call.
//!
//! We parse with `typst-syntax` (no compilation) so this is fast and
//! side-effect-free.

use crate::{FileMeta, Format, ValidationError};
use typst_syntax::{ast, parse, SyntaxKind, SyntaxNode};

use super::TYPST_LABEL;

#[derive(Debug, Default)]
struct Raw {
    lang: Option<String>,
    translation_of: Option<String>,
    title: Option<String>,
    abstract_: Option<String>,
    tags: Vec<String>,
    translator: Option<String>,
    translation_notes: Option<String>,
}

pub fn extract(path: &str, content: &str) -> Result<FileMeta, ValidationError> {
    let root = parse(content);
    let dict_node = find_labelled_metadata(&root).ok_or_else(|| ValidationError::MetadataMissing {
        path: path.to_string(),
    })?;

    let raw = parse_dict(path, dict_node)?;

    let lang = raw.lang.ok_or_else(|| ValidationError::MissingLang {
        path: path.to_string(),
    })?;

    Ok(FileMeta {
        path: path.to_string(),
        format: Format::Typst,
        lang,
        translation_of: raw.translation_of,
        title: raw.title,
        abstract_: raw.abstract_,
        tags: raw.tags,
        translator: raw.translator,
        translation_notes: raw.translation_notes,
    })
}

/// Walk the AST looking for `#metadata(<dict>) <nightboat-translation>`.
/// Returns a reference to the `<dict>` node.
fn find_labelled_metadata(root: &SyntaxNode) -> Option<&SyntaxNode> {
    walk_nodes(root).find_map(|(node, siblings_after)| {
        let call = node.cast::<ast::FuncCall>()?;
        let callee = call.callee();
        let ast::Expr::Ident(ident) = callee else {
            return None;
        };
        if ident.as_str() != "metadata" {
            return None;
        }
        if !followed_by_our_label(&siblings_after) {
            return None;
        }
        // The arg must be a single positional dict.
        let args = call.args();
        let mut positional = args.items().filter_map(|item| match item {
            ast::Arg::Pos(expr) => Some(expr),
            _ => None,
        });
        let dict_expr = positional.next()?;
        if positional.next().is_some() {
            return None;
        }
        let ast::Expr::Dict(_) = dict_expr else {
            return None;
        };
        // Return the underlying SyntaxNode of the dict.
        node_of_expr(node, &dict_expr)
    })
}

/// Given the parent FuncCall node, find the child SyntaxNode that corresponds
/// to the given dict expression. We simply look for the first Dict child.
fn node_of_expr<'a>(call_node: &'a SyntaxNode, _expr: &ast::Expr<'_>) -> Option<&'a SyntaxNode> {
    // Walk descendants to find a Dict node. Robust enough because our contract
    // is "single positional dict arg".
    fn first_dict(node: &SyntaxNode) -> Option<&SyntaxNode> {
        if node.kind() == SyntaxKind::Dict {
            return Some(node);
        }
        for child in node.children() {
            if let Some(d) = first_dict(child) {
                return Some(d);
            }
        }
        None
    }
    first_dict(call_node)
}

/// Returns true if among the siblings immediately following the metadata call
/// (skipping whitespace and parbreaks) the first non-trivial node is the label
/// `<nightboat-translation>`.
fn followed_by_our_label(siblings_after: &[&SyntaxNode]) -> bool {
    for n in siblings_after {
        match n.kind() {
            SyntaxKind::Space | SyntaxKind::Parbreak => continue,
            SyntaxKind::Label => {
                // label text includes angle brackets: `<nightboat-translation>`
                return n.text().as_str() == format!("<{}>", TYPST_LABEL);
            }
            _ => return false,
        }
    }
    false
}

/// Depth-first walk yielding `(node, siblings_after_this_node_under_same_parent)`.
fn walk_nodes(root: &SyntaxNode) -> impl Iterator<Item = (&SyntaxNode, Vec<&SyntaxNode>)> {
    // Build a vec via recursion since typst-syntax doesn't expose a built-in walker.
    let mut out: Vec<(&SyntaxNode, Vec<&SyntaxNode>)> = Vec::new();
    fn rec<'a>(node: &'a SyntaxNode, out: &mut Vec<(&'a SyntaxNode, Vec<&'a SyntaxNode>)>) {
        let kids: Vec<&SyntaxNode> = node.children().collect();
        for (i, child) in kids.iter().enumerate() {
            let siblings_after: Vec<&SyntaxNode> = kids[i + 1..].iter().copied().collect();
            out.push((*child, siblings_after));
            rec(child, out);
        }
    }
    rec(root, &mut out);
    out.into_iter()
}

fn parse_dict(path: &str, dict_node: &SyntaxNode) -> Result<Raw, ValidationError> {
    let dict = dict_node
        .cast::<ast::Dict>()
        .ok_or_else(|| ValidationError::MetadataParseError {
            path: path.to_string(),
            reason: "metadata argument is not a dict literal".into(),
        })?;

    let mut raw = Raw::default();

    for item in dict.items() {
        match item {
            ast::DictItem::Named(named) => {
                let key = named.name().as_str().to_string();
                match key.as_str() {
                    "lang" => raw.lang = Some(expect_string(path, &key, named.expr())?),
                    "translation-of" | "translation_of" => {
                        raw.translation_of = Some(expect_string(path, &key, named.expr())?)
                    }
                    "title" => raw.title = Some(expect_string(path, &key, named.expr())?),
                    "abstract" => raw.abstract_ = Some(expect_string(path, &key, named.expr())?),
                    "translator" => {
                        raw.translator = Some(expect_string(path, &key, named.expr())?)
                    }
                    "translation-notes" | "translation_notes" => {
                        raw.translation_notes =
                            Some(expect_string(path, &key, named.expr())?)
                    }
                    "tags" => raw.tags = expect_string_array(path, &key, named.expr())?,
                    other => {
                        return Err(ValidationError::MetadataParseError {
                            path: path.to_string(),
                            reason: format!("unknown metadata key: {other:?}"),
                        });
                    }
                }
            }
            ast::DictItem::Keyed(_) | ast::DictItem::Spread(_) => {
                return Err(ValidationError::MetadataParseError {
                    path: path.to_string(),
                    reason: "metadata dict must only use named fields (key: value)".into(),
                });
            }
        }
    }

    Ok(raw)
}

fn expect_string(path: &str, key: &str, expr: ast::Expr<'_>) -> Result<String, ValidationError> {
    match expr {
        ast::Expr::Str(s) => Ok(s.get().to_string()),
        _ => Err(ValidationError::MetadataParseError {
            path: path.to_string(),
            reason: format!("field {key:?} must be a string"),
        }),
    }
}

fn expect_string_array(
    path: &str,
    key: &str,
    expr: ast::Expr<'_>,
) -> Result<Vec<String>, ValidationError> {
    let ast::Expr::Array(arr) = expr else {
        return Err(ValidationError::MetadataParseError {
            path: path.to_string(),
            reason: format!("field {key:?} must be an array of strings"),
        });
    };
    let mut out = Vec::new();
    for item in arr.items() {
        let ast::ArrayItem::Pos(ast::Expr::Str(s)) = item else {
            return Err(ValidationError::MetadataParseError {
                path: path.to_string(),
                reason: format!("field {key:?} must contain only string literals"),
            });
        };
        out.push(s.get().to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_source_metadata() {
        let src = r#"
#metadata((
  lang: "en",
  title: "Quantum Intro",
)) <nightboat-translation>

= Body
"#;
        let meta = extract("quantum-intro/main.typ", src).unwrap();
        assert_eq!(meta.lang, "en");
        assert_eq!(meta.title.as_deref(), Some("Quantum Intro"));
        assert_eq!(meta.translation_of, None);
    }

    #[test]
    fn extracts_translation_metadata() {
        let src = r#"
#metadata((
  lang: "zh-CN",
  translation-of: "../quantum-intro/main.typ",
  title: "量子入门",
  tags: ("physics", "qm"),
  translator: "did:plc:abc",
)) <nightboat-translation>
"#;
        let meta = extract("quantum-intro.zh-CN/main.typ", src).unwrap();
        assert_eq!(meta.lang, "zh-CN");
        assert_eq!(
            meta.translation_of.as_deref(),
            Some("../quantum-intro/main.typ")
        );
        assert_eq!(meta.tags, vec!["physics", "qm"]);
    }

    #[test]
    fn rejects_missing_label() {
        let src = r#"#metadata((lang: "en"))"#;
        assert!(matches!(
            extract("main.typ", src).unwrap_err(),
            ValidationError::MetadataMissing { .. }
        ));
    }

    #[test]
    fn rejects_non_string_value() {
        let src = r#"#metadata((lang: 42)) <nightboat-translation>"#;
        assert!(matches!(
            extract("main.typ", src).unwrap_err(),
            ValidationError::MetadataParseError { .. }
        ));
    }

    #[test]
    fn rejects_unknown_key() {
        let src = r#"#metadata((lang: "en", weird: "x")) <nightboat-translation>"#;
        assert!(matches!(
            extract("main.typ", src).unwrap_err(),
            ValidationError::MetadataParseError { .. }
        ));
    }

    #[test]
    fn missing_lang_is_caught() {
        let src = r#"#metadata((title: "X")) <nightboat-translation>"#;
        assert!(matches!(
            extract("main.typ", src).unwrap_err(),
            ValidationError::MissingLang { .. }
        ));
    }
}
