//! Filesystem scan: walk a directory tree and extract `FileMeta` for every
//! article file encountered. Pure IO + `extract::*`; returns data the caller
//! feeds to [`crate::validate_files`].
//!
//! This is the one entry point the knot's pre-receive hook needs: point it at
//! the working copy that would result from applying the incoming change, run
//! `validate_files` on the result, reject the push if the report is non-empty.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{extract, FileMeta, ValidationError};

/// Directory components that are never article content; skipped during the
/// walk to avoid spurious errors on pijul internals, rendered artifacts, and
/// the typst package cache.
const SKIP_DIR_NAMES: &[&str] = &[
    ".pijul",
    ".cache",
    "cache",
    "_rendered",
    "typst-packages",
];

/// One file that couldn't be read or parsed during the scan.
#[derive(Debug, Clone)]
pub struct ScanError {
    pub path: PathBuf,
    pub error: ScanErrorKind,
}

#[derive(Debug, Clone)]
pub enum ScanErrorKind {
    /// Failed to read the file from disk (permissions, not-found, etc.).
    Io(String),
    /// Extraction returned a validator error.
    Extract(ValidationError),
}

/// Walk `root` and return a `FileMeta` for every article file found, plus any
/// per-file errors encountered. Cross-file validation is **not** performed
/// here — call [`crate::validate_files`] on the returned `Vec<FileMeta>` for
/// that.
pub fn scan_dir(root: &Path) -> (Vec<FileMeta>, Vec<ScanError>) {
    let mut metas: Vec<FileMeta> = Vec::new();
    let mut errors: Vec<ScanError> = Vec::new();
    visit(root, root, &mut metas, &mut errors);
    (metas, errors)
}

fn visit(root: &Path, dir: &Path, metas: &mut Vec<FileMeta>, errors: &mut Vec<ScanError>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) => {
            errors.push(ScanError {
                path: dir.to_path_buf(),
                error: ScanErrorKind::Io(err.to_string()),
            });
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if SKIP_DIR_NAMES.contains(&name_str.as_ref()) || name_str.starts_with('.') {
            continue;
        }
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(err) => {
                errors.push(ScanError {
                    path: path.clone(),
                    error: ScanErrorKind::Io(err.to_string()),
                });
                continue;
            }
        };
        if file_type.is_dir() {
            visit(root, &path, metas, errors);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        // Relative path from root, POSIX-normalized (forward slashes).
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel_str: String = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");

        if !extract::is_article_file(&rel_str) {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(err) => {
                errors.push(ScanError {
                    path: path.clone(),
                    error: ScanErrorKind::Io(err.to_string()),
                });
                continue;
            }
        };

        match extract::extract(&rel_str, &content) {
            Ok(meta) => metas.push(meta),
            Err(err) => errors.push(ScanError {
                path: path.clone(),
                error: ScanErrorKind::Extract(err),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(dir: &Path, rel: &str, content: &str) {
        let path = dir.join(rel);
        if let Some(p) = path.parent() {
            fs::create_dir_all(p).unwrap();
        }
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn scan_walks_tree_and_skips_internals() {
        let dir = tempfile::TempDir::new().unwrap();
        write_file(
            dir.path(),
            "a.md",
            "---\nlang: en\ntitle: A\n---\n",
        );
        write_file(
            dir.path(),
            "sub/b.md",
            "---\nlang: zh-CN\ntranslation-of: ../a.md\n---\n",
        );
        write_file(dir.path(), ".pijul/junk.md", "---\nlang: en\n---\n");
        write_file(dir.path(), "cache/ignored.md", "---\nlang: en\n---\n");

        let (metas, errors) = scan_dir(dir.path());
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        let paths: Vec<_> = metas.iter().map(|m| m.path.as_str()).collect();
        assert!(paths.contains(&"a.md"));
        assert!(paths.contains(&"sub/b.md"));
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn scan_reports_parse_errors() {
        let dir = tempfile::TempDir::new().unwrap();
        write_file(dir.path(), "ok.md", "---\nlang: en\n---\n");
        write_file(dir.path(), "broken.md", "no frontmatter\n");

        let (metas, errors) = scan_dir(dir.path());
        assert_eq!(metas.len(), 1);
        assert_eq!(errors.len(), 1);
        match &errors[0].error {
            ScanErrorKind::Extract(ValidationError::MetadataMissing { .. }) => {}
            other => panic!("unexpected error kind: {other:?}"),
        }
    }
}
