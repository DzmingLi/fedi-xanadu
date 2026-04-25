//! POSIX path normalization for repo-relative paths.
//!
//! We deliberately avoid `std::path::Path` because pijul repos are POSIX-style
//! across all platforms (the on-disk representation in the repo is the same on
//! Linux, macOS, Windows). Mixing in OS-dependent separators would break the
//! primary-key determinism of `(repo_did, source_path)`.

/// Error from path normalization.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum PathError {
    #[error("absolute paths not allowed in repo metadata")]
    Absolute,
    #[error("path escapes repository root")]
    EscapesRoot,
    #[error("empty path")]
    Empty,
    #[error("backslashes not allowed; use forward slashes")]
    Backslash,
}

/// Normalize a repo-relative path. Accepts `./foo`, `foo/./bar`, etc.
/// Rejects `..` that escapes root, absolute paths, and backslashes.
pub fn normalize_repo_path(input: &str) -> Result<String, PathError> {
    if input.contains('\\') {
        return Err(PathError::Backslash);
    }
    if input.starts_with('/') {
        return Err(PathError::Absolute);
    }
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(PathError::Empty);
    }

    let mut stack: Vec<&str> = Vec::new();
    for seg in trimmed.split('/') {
        match seg {
            "" | "." => continue,
            ".." => {
                if stack.pop().is_none() {
                    return Err(PathError::EscapesRoot);
                }
            }
            s => stack.push(s),
        }
    }
    if stack.is_empty() {
        return Err(PathError::Empty);
    }
    Ok(stack.join("/"))
}

/// Resolve `reference` (as written in a translation file) against `from_file`
/// (repo-relative path of the translation itself). Returns a normalized
/// repo-relative path.
///
/// Example: `from_file = "a/b/zh.md"`, `reference = "../source.md"` →
/// `"a/source.md"`.
pub fn resolve_relative(from_file: &str, reference: &str) -> Result<String, PathError> {
    if reference.starts_with('/') {
        // Treat as repo-absolute (rooted). Strip leading slash and normalize.
        return normalize_repo_path(&reference[1..]);
    }
    let parent = parent_of(from_file);
    let joined = if parent.is_empty() {
        reference.to_string()
    } else {
        format!("{parent}/{reference}")
    };
    normalize_repo_path(&joined)
}

fn parent_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_basic() {
        assert_eq!(normalize_repo_path("foo/bar.md").unwrap(), "foo/bar.md");
        assert_eq!(normalize_repo_path("./foo/bar.md").unwrap(), "foo/bar.md");
        assert_eq!(normalize_repo_path("foo//bar.md").unwrap(), "foo/bar.md");
        assert_eq!(normalize_repo_path("foo/./bar.md").unwrap(), "foo/bar.md");
        assert_eq!(normalize_repo_path("foo/../bar.md").unwrap(), "bar.md");
    }

    #[test]
    fn normalize_rejects_escape() {
        assert_eq!(normalize_repo_path("/abs/path"), Err(PathError::Absolute));
        assert_eq!(normalize_repo_path("../x"), Err(PathError::EscapesRoot));
        assert_eq!(
            normalize_repo_path("a/../../x"),
            Err(PathError::EscapesRoot)
        );
        assert_eq!(normalize_repo_path(""), Err(PathError::Empty));
        assert_eq!(normalize_repo_path("   "), Err(PathError::Empty));
        assert_eq!(
            normalize_repo_path("win\\path"),
            Err(PathError::Backslash)
        );
    }

    #[test]
    fn resolve_sibling_translation() {
        assert_eq!(
            resolve_relative("quantum-intro.zh-CN.md", "./quantum-intro.md").unwrap(),
            "quantum-intro.md"
        );
    }

    #[test]
    fn resolve_cross_dir_translation() {
        // Typst case: translation in its own directory, source in sibling dir.
        assert_eq!(
            resolve_relative(
                "quantum-intro.zh-CN/main.typ",
                "../quantum-intro/main.typ"
            )
            .unwrap(),
            "quantum-intro/main.typ"
        );
    }

    #[test]
    fn resolve_rejects_escape() {
        assert!(resolve_relative("a.md", "../../../etc/passwd").is_err());
    }
}
