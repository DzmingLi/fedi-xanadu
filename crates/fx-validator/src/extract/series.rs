//! Read series-level metadata from a bundle.
//!
//! Two slots per the `at.nightbo.work` lexicon:
//! - typst series: `<nbt-series>` directive at the top of `main.typ`
//! - other series: `meta.yml` at the bundle root
//!
//! Either way the result is the same shape; the publish path picks the
//! slot based on format. The re-index path tries main.typ first; if the
//! `<nbt-series>` directive isn't there, it falls back to `meta.yml`.

use crate::ValidationError;
use serde::{Deserialize, Serialize};
use typst_syntax::{ast, parse, SyntaxKind, SyntaxNode};

/// Series-level metadata fields.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesMeta {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, alias = "abstract")]
    pub description: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
}

/// Parse `meta.yml` content (YAML) at a series's bundle root.
pub fn extract_meta_yml(path: &str, yaml: &str) -> Result<SeriesMeta, ValidationError> {
    serde_yml::from_str(yaml).map_err(|e| ValidationError::MetadataParseError {
        path: path.to_string(),
        reason: format!("meta.yml parse error: {e}"),
    })
}

/// Parse the `<nbt-series>` directive at the top of a typst `main.typ`.
/// Returns `Ok(None)` when the directive is absent (caller can fall back to
/// `meta.yml`).
pub fn extract_typst_main(path: &str, content: &str) -> Result<Option<SeriesMeta>, ValidationError> {
    let root = parse(content);
    let Some(dict) = find_series_dict(&root) else { return Ok(None) };
    let mut out = SeriesMeta::default();
    let dict = dict.cast::<ast::Dict>().ok_or_else(|| ValidationError::MetadataParseError {
        path: path.to_string(),
        reason: "metadata argument is not a dict literal".into(),
    })?;
    for item in dict.items() {
        let ast::DictItem::Named(named) = item else {
            return Err(ValidationError::MetadataParseError {
                path: path.to_string(),
                reason: "metadata dict must only use named fields".into(),
            });
        };
        let key = named.name().as_str().to_string();
        match key.as_str() {
            "title" => out.title = Some(string_value(path, &key, named.expr())?),
            "abstract" | "description" => out.description = Some(string_value(path, &key, named.expr())?),
            "lang" => out.lang = Some(string_value(path, &key, named.expr())?),
            "category" => out.category = Some(string_value(path, &key, named.expr())?),
            "topics" | "tags" => out.topics = string_array(path, &key, named.expr())?,
            other => return Err(ValidationError::MetadataParseError {
                path: path.to_string(),
                reason: format!("unknown series-metadata key: {other:?}"),
            }),
        }
    }
    Ok(Some(out))
}

const NBT_SERIES_LABEL: &str = "nbt-series";

fn find_series_dict(root: &SyntaxNode) -> Option<&SyntaxNode> {
    fn walk<'a>(node: &'a SyntaxNode, out: &mut Vec<(&'a SyntaxNode, Vec<&'a SyntaxNode>)>) {
        let kids: Vec<&SyntaxNode> = node.children().collect();
        for (i, child) in kids.iter().enumerate() {
            let after: Vec<&SyntaxNode> = kids[i + 1..].iter().copied().collect();
            out.push((*child, after));
            walk(child, out);
        }
    }
    let mut nodes: Vec<(&SyntaxNode, Vec<&SyntaxNode>)> = Vec::new();
    walk(root, &mut nodes);
    nodes.into_iter().find_map(|(node, after)| {
        let call = node.cast::<ast::FuncCall>()?;
        let ast::Expr::Ident(ident) = call.callee() else { return None };
        if ident.as_str() != "metadata" { return None }
        if !label_matches(&after, NBT_SERIES_LABEL) { return None }
        first_dict(node)
    })
}

fn label_matches(after: &[&SyntaxNode], label: &str) -> bool {
    for n in after {
        match n.kind() {
            SyntaxKind::Space | SyntaxKind::Parbreak => continue,
            SyntaxKind::Label => return n.text().as_str() == format!("<{label}>"),
            _ => return false,
        }
    }
    false
}

fn first_dict(node: &SyntaxNode) -> Option<&SyntaxNode> {
    if node.kind() == SyntaxKind::Dict { return Some(node) }
    for c in node.children() {
        if let Some(d) = first_dict(c) { return Some(d) }
    }
    None
}

fn string_value(path: &str, key: &str, expr: ast::Expr<'_>) -> Result<String, ValidationError> {
    match expr {
        ast::Expr::Str(s) => Ok(s.get().to_string()),
        _ => Err(ValidationError::MetadataParseError {
            path: path.to_string(),
            reason: format!("field {key:?} must be a string"),
        }),
    }
}

fn string_array(path: &str, key: &str, expr: ast::Expr<'_>) -> Result<Vec<String>, ValidationError> {
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
    fn parses_meta_yml() {
        let yaml = "title: \"Static Analysis\"\ndescription: \"Course notes\"\nlang: zh\ncategory: lecture\ntopics:\n  - static-analysis\n  - compiler\n";
        let m = extract_meta_yml("meta.yml", yaml).unwrap();
        assert_eq!(m.title.as_deref(), Some("Static Analysis"));
        assert_eq!(m.description.as_deref(), Some("Course notes"));
        assert_eq!(m.lang.as_deref(), Some("zh"));
        assert_eq!(m.category.as_deref(), Some("lecture"));
        assert_eq!(m.topics, vec!["static-analysis".to_string(), "compiler".to_string()]);
    }

    #[test]
    fn parses_typst_nbt_series() {
        let src = r#"
#metadata((
  title: "Static Analysis",
  description: "Course notes",
  lang: "zh",
  category: "lecture",
  topics: ("static-analysis", "compiler"),
)) <nbt-series>

= Outline
"#;
        let m = extract_typst_main("main.typ", src).unwrap().expect("found");
        assert_eq!(m.title.as_deref(), Some("Static Analysis"));
        assert_eq!(m.description.as_deref(), Some("Course notes"));
        assert_eq!(m.lang.as_deref(), Some("zh"));
        assert_eq!(m.topics, vec!["static-analysis".to_string(), "compiler".to_string()]);
    }

    #[test]
    fn typst_main_without_nbt_series_returns_none() {
        let src = "= Body\n\nNo metadata directive here.\n";
        assert!(extract_typst_main("main.typ", src).unwrap().is_none());
    }

    #[test]
    fn typst_main_round_trip_via_inject() {
        use crate::inject::{Metadata, typst as itypst};
        let m = Metadata {
            title: Some("S".into()),
            abstract_: Some("D".into()),
            lang: Some("zh".into()),
            category: Some("lecture".into()),
            tags: vec!["t1".into(), "t2".into()],
            ..Default::default()
        };
        let injected = itypst::merge_series("= Body\n", &m);
        let out = extract_typst_main("main.typ", &injected).unwrap().expect("found");
        assert_eq!(out.title.as_deref(), Some("S"));
        assert_eq!(out.description.as_deref(), Some("D"));
        assert_eq!(out.lang.as_deref(), Some("zh"));
        assert_eq!(out.category.as_deref(), Some("lecture"));
        assert_eq!(out.topics, vec!["t1".to_string(), "t2".to_string()]);
    }
}
