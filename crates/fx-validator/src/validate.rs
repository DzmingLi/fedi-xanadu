//! Cross-file validation: groups files into articles and enforces invariants.

use crate::{
    path::resolve_relative, Article, ArticleSet, FileMap, FileMeta, Localization,
    ValidationError, ValidationReport,
};
use std::collections::BTreeMap;

/// Validate a set of pre-extracted file metadata.
///
/// Returns:
/// - `ArticleSet` holding the successfully grouped articles (may be partial
///   when there are errors — callers decide whether to accept the partial
///   result or reject the whole push).
/// - `ValidationReport` containing all errors encountered.
pub fn validate_files(files: Vec<FileMeta>) -> (ArticleSet, ValidationReport) {
    let mut report = ValidationReport::default();

    // Build path map, catching pathlevel issues (lang tag etc.) as we go.
    let mut by_path: FileMap = BTreeMap::new();
    for f in files {
        if let Err(err) = check_single(&f) {
            report.push(err);
            continue;
        }
        if by_path.insert(f.path.clone(), f).is_some() {
            // Duplicate path shouldn't happen (one file = one extraction) but
            // guard anyway to keep callers honest.
            let dup = by_path.keys().last().cloned().unwrap_or_default();
            report.push(ValidationError::MetadataParseError {
                path: dup,
                reason: "file visited twice".into(),
            });
        }
    }

    // Partition into sources and translations.
    let mut sources: BTreeMap<String, FileMeta> = BTreeMap::new();
    let mut translations: Vec<(FileMeta, String)> = Vec::new(); // (file, resolved_source_path)

    for (path, meta) in &by_path {
        match &meta.translation_of {
            None => {
                sources.insert(path.clone(), meta.clone());
            }
            Some(rel) => match resolve_relative(path, rel) {
                Ok(resolved) => translations.push((meta.clone(), resolved)),
                Err(e) => report.push(ValidationError::InvalidPath {
                    path: path.clone(),
                    reason: format!("translation_of ({rel:?}): {e}"),
                }),
            },
        }
    }

    // Assemble localizations per source.
    let mut localizations: BTreeMap<String, Vec<FileMeta>> = BTreeMap::new();
    for (src_path, src) in &sources {
        localizations.insert(src_path.clone(), vec![src.clone()]);
    }

    for (trans, resolved) in translations {
        let trans_path = trans.path.clone();

        // Check target exists.
        let Some(target_meta) = by_path.get(&resolved) else {
            report.push(ValidationError::TranslationTargetMissing {
                translation: trans_path,
                target: resolved,
            });
            continue;
        };
        // Reject chains: target must itself be a source.
        if target_meta.translation_of.is_some() {
            report.push(ValidationError::TranslationChain {
                translation: trans_path,
                intermediate: resolved,
            });
            continue;
        }

        localizations
            .entry(resolved)
            .or_default()
            .push(trans);
    }

    // Build articles, checking for duplicate langs.
    let mut articles: Vec<Article> = Vec::new();
    for (source_path, mut locs) in localizations {
        let source_meta = match sources.get(&source_path) {
            Some(s) => s,
            None => continue, // defensive: shouldn't happen since every key came from sources.
        };

        // Check duplicate langs.
        let mut by_lang: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for l in &locs {
            by_lang.entry(l.lang.clone()).or_default().push(l.path.clone());
        }
        let mut has_dup = false;
        for (lang, files) in &by_lang {
            if files.len() > 1 {
                has_dup = true;
                report.push(ValidationError::DuplicateLanguage {
                    source_path: source_path.clone(),
                    lang: lang.clone(),
                    files: files.clone(),
                });
            }
        }
        if has_dup {
            // Skip emitting an article with conflicting langs — author must fix.
            continue;
        }

        // Sort localizations by lang for determinism.
        locs.sort_by(|a, b| a.lang.cmp(&b.lang));

        let localizations = locs
            .into_iter()
            .map(|f| Localization {
                lang: f.lang,
                file_path: f.path,
                title: f.title,
                abstract_: f.abstract_,
                tags: f.tags,
                translator: f.translator,
                translation_notes: f.translation_notes,
            })
            .collect();

        articles.push(Article {
            source_path: source_path.clone(),
            source_lang: source_meta.lang.clone(),
            localizations,
        });
    }

    articles.sort_by(|a, b| a.source_path.cmp(&b.source_path));
    (ArticleSet { articles }, report)
}

/// Convenience wrapper that returns `Ok` when the report is clean, `Err`
/// otherwise. Use when partial results are not interesting (e.g. pre-receive
/// hook: all or nothing).
pub fn validate(files: Vec<FileMeta>) -> Result<ArticleSet, ValidationReport> {
    let (set, report) = validate_files(files);
    if report.is_ok() {
        Ok(set)
    } else {
        Err(report)
    }
}

/// Per-file checks that don't depend on sibling files.
fn check_single(f: &FileMeta) -> Result<(), ValidationError> {
    crate::path::normalize_repo_path(&f.path)
        .map_err(|e| ValidationError::InvalidPath {
            path: f.path.clone(),
            reason: e.to_string(),
        })
        .and_then(|norm| {
            if norm == f.path {
                Ok(())
            } else {
                Err(ValidationError::InvalidPath {
                    path: f.path.clone(),
                    reason: format!("path is not normalized (expected {norm:?})"),
                })
            }
        })?;

    if !is_valid_bcp47(&f.lang) {
        return Err(ValidationError::InvalidLang {
            path: f.path.clone(),
            lang: f.lang.clone(),
        });
    }

    Ok(())
}

/// Minimal BCP 47 check: non-empty, ASCII, `[A-Za-z0-9-]+`, no leading/trailing
/// hyphens, no double hyphens. Full RFC 5646 validation would be overkill for
/// now; catch obvious typos.
fn is_valid_bcp47(tag: &str) -> bool {
    if tag.is_empty() || tag.starts_with('-') || tag.ends_with('-') || tag.contains("--") {
        return false;
    }
    tag.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Format;

    fn src(path: &str, lang: &str) -> FileMeta {
        FileMeta {
            path: path.into(),
            format: Format::Md,
            lang: lang.into(),
            ..Default::default()
        }
    }

    fn tr(path: &str, lang: &str, of: &str) -> FileMeta {
        FileMeta {
            translation_of: Some(of.into()),
            ..src(path, lang)
        }
    }

    #[test]
    fn simple_source_only() {
        let (set, report) = validate_files(vec![src("a.md", "en")]);
        assert!(report.is_ok());
        assert_eq!(set.articles.len(), 1);
        assert_eq!(set.articles[0].source_lang, "en");
        assert_eq!(set.articles[0].localizations.len(), 1);
    }

    #[test]
    fn source_with_translation() {
        let (set, report) = validate_files(vec![
            src("quantum.md", "en"),
            tr("quantum.zh-CN.md", "zh-CN", "./quantum.md"),
        ]);
        assert!(report.is_ok(), "{:?}", report.errors);
        assert_eq!(set.articles.len(), 1);
        assert_eq!(set.articles[0].localizations.len(), 2);
        let langs: Vec<_> = set.articles[0]
            .localizations
            .iter()
            .map(|l| l.lang.as_str())
            .collect();
        assert_eq!(langs, vec!["en", "zh-CN"]);
    }

    #[test]
    fn translation_missing_target() {
        let (_set, report) = validate_files(vec![tr("z.md", "zh", "./missing.md")]);
        assert_eq!(report.errors.len(), 1);
        assert!(matches!(
            report.errors[0],
            ValidationError::TranslationTargetMissing { .. }
        ));
    }

    #[test]
    fn translation_chain_forbidden() {
        let (_set, report) = validate_files(vec![
            src("a.md", "en"),
            tr("b.md", "fr", "./a.md"),
            tr("c.md", "zh", "./b.md"),
        ]);
        assert!(report
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::TranslationChain { .. })));
    }

    #[test]
    fn duplicate_lang_error() {
        let (_set, report) = validate_files(vec![
            src("a.md", "en"),
            tr("b.md", "en", "./a.md"),
        ]);
        assert!(report
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::DuplicateLanguage { .. })));
    }

    #[test]
    fn cross_dir_translation() {
        // Typst-style layout.
        let (set, report) = validate_files(vec![
            src("quantum/main.typ", "en"),
            tr("quantum.zh-CN/main.typ", "zh-CN", "../quantum/main.typ"),
        ]);
        assert!(report.is_ok(), "{:?}", report.errors);
        assert_eq!(set.articles.len(), 1);
        assert_eq!(set.articles[0].localizations.len(), 2);
    }

    #[test]
    fn invalid_path_not_normalized() {
        let bad = FileMeta {
            path: "./foo.md".into(), // not normalized — should fail
            ..src("foo.md", "en")
        };
        let (_set, report) = validate_files(vec![bad]);
        assert!(report
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidPath { .. })));
    }

    #[test]
    fn invalid_lang_rejected() {
        let (_set, report) = validate_files(vec![src("a.md", "--bad")]);
        assert!(report
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidLang { .. })));
    }
}
