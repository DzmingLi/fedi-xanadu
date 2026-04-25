//! Scan a pijul working copy, extract article metadata, and upsert it into
//! the database.
//!
//! One pijul repo can contain one (standalone) or many (series) logical
//! articles, each of which may have several language localizations declared
//! via file-level metadata (`lang` + `translation_of`). The indexer is
//! idempotent: re-running it after a pijul push reconciles the DB with the
//! current repo contents (rows for files that disappeared are deleted).
//!
//! Called from fx-server after a successful `knot.record` — it's the bridge
//! between "content is in pijul" and "content is queryable in PostgreSQL".

use std::path::Path;

use fx_validator::{scan, validate_files, Article as VArticle, FileMeta, ValidationReport};
use serde::Serialize;
use sqlx::PgPool;

/// Summary of what the indexer did during a single run.
#[derive(Debug, Default, Clone, Serialize)]
pub struct IndexReport {
    /// Number of articles upserted into `articles`.
    pub articles_upserted: usize,
    /// Number of localizations upserted into `article_localizations`.
    pub localizations_upserted: usize,
    /// Localization rows removed because their file disappeared from the repo.
    pub localizations_removed: usize,
    /// Validation errors encountered. The indexer is tolerant: valid articles
    /// are still upserted even when others fail.
    pub errors: ValidationReport,
}

/// Scan `repo_path` for article files, extract metadata, validate, and
/// upsert into the DB under `repo_uri`. `default_author_did` is the DID
/// attributed to new articles (normally the repo owner).
pub async fn index_repo(
    pool: &PgPool,
    repo_uri: &str,
    repo_path: &Path,
    default_author_did: &str,
) -> crate::Result<IndexReport> {
    let mut report = IndexReport::default();

    // --- 1. Walk the working copy, extracting FileMeta from each article file.
    let metas = collect_file_metas(repo_path, &mut report);
    tracing::debug!(
        "indexer: scanned {} file(s) in {}",
        metas.len(),
        repo_path.display()
    );

    // --- 2. Cross-file validation (translation chains, duplicate langs, etc.)
    let (article_set, report_validation) = validate_files(metas);
    report.errors.errors.extend(report_validation.errors);

    // --- 3. Upsert each validated article + its localizations.
    let mut tx = pool.begin().await?;

    // Track the (source_path, lang) pairs we saw, to clean up rows that no
    // longer exist in the repo.
    let mut seen: Vec<(String, String)> = Vec::new();

    for article in &article_set.articles {
        upsert_article(&mut tx, repo_uri, article, default_author_did).await?;
        report.articles_upserted += 1;

        for loc in &article.localizations {
            upsert_localization(&mut tx, repo_uri, article, loc).await?;
            report.localizations_upserted += 1;
            seen.push((article.source_path.clone(), loc.lang.clone()));
        }
    }

    // --- 4. Delete rows for source_paths / langs the current scan did not see.
    report.localizations_removed =
        prune_missing_rows(&mut tx, repo_uri, &seen).await?;

    tx.commit().await?;
    Ok(report)
}

fn collect_file_metas(repo_path: &Path, report: &mut IndexReport) -> Vec<FileMeta> {
    let (metas, scan_errors) = scan::scan_dir(repo_path);
    for err in scan_errors {
        match err.error {
            scan::ScanErrorKind::Extract(e) => {
                tracing::debug!("indexer: extract error on {}: {e}", err.path.display());
                report.errors.push(e);
            }
            scan::ScanErrorKind::Io(msg) => {
                tracing::warn!("indexer: io error on {}: {msg}", err.path.display());
            }
        }
    }
    metas
}

async fn upsert_article(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    repo_uri: &str,
    article: &VArticle,
    default_author_did: &str,
) -> crate::Result<()> {
    // Article-level columns are stable across re-indexing; we only set
    // license / author_did on INSERT. On subsequent indexing passes we leave
    // them alone (an admin or the author may have edited them separately).
    sqlx::query(
        "INSERT INTO articles (repo_uri, source_path, author_did) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (repo_uri, source_path) DO UPDATE SET updated_at = NOW()",
    )
    .bind(repo_uri)
    .bind(&article.source_path)
    .bind(default_author_did)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_localization(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    repo_uri: &str,
    article: &VArticle,
    loc: &fx_validator::Localization,
) -> crate::Result<()> {
    let content_format = infer_content_format(&loc.file_path);
    let title = loc.title.clone().unwrap_or_default();
    let summary = loc.abstract_.clone().unwrap_or_default();
    sqlx::query(
        "INSERT INTO article_localizations ( \
             repo_uri, source_path, lang, file_path, \
             content_format, title, summary, translator_did, translation_notes \
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (repo_uri, source_path, lang) DO UPDATE SET \
             file_path = EXCLUDED.file_path, \
             content_format = EXCLUDED.content_format, \
             title = EXCLUDED.title, \
             summary = EXCLUDED.summary, \
             translator_did = EXCLUDED.translator_did, \
             translation_notes = EXCLUDED.translation_notes, \
             updated_at = NOW()",
    )
    .bind(repo_uri)
    .bind(&article.source_path)
    .bind(&loc.lang)
    .bind(&loc.file_path)
    .bind(content_format)
    .bind(&title)
    .bind(&summary)
    .bind(&loc.translator)
    .bind(&loc.translation_notes)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn infer_content_format(file_path: &str) -> &'static str {
    if file_path.ends_with(".md") {
        "markdown"
    } else if file_path.ends_with(".html") {
        "html"
    } else {
        "typst"
    }
}

/// Delete localization rows under this repo whose (source_path, lang) pair
/// did not appear in the current scan. Cascade-cleans the `articles` row
/// when all of its localizations are gone.
async fn prune_missing_rows(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    repo_uri: &str,
    seen: &[(String, String)],
) -> crate::Result<usize> {
    // Collect what's currently in DB.
    let existing: Vec<(String, String)> = sqlx::query_as(
        "SELECT source_path, lang FROM article_localizations WHERE repo_uri = $1",
    )
    .bind(repo_uri)
    .fetch_all(&mut **tx)
    .await?;

    let mut to_delete: Vec<(String, String)> = Vec::new();
    for row in &existing {
        if !seen.iter().any(|s| s == row) {
            to_delete.push(row.clone());
        }
    }

    let mut removed = 0;
    for (source_path, lang) in &to_delete {
        let r = sqlx::query(
            "DELETE FROM article_localizations \
             WHERE repo_uri = $1 AND source_path = $2 AND lang = $3",
        )
        .bind(repo_uri)
        .bind(source_path)
        .bind(lang)
        .execute(&mut **tx)
        .await?;
        removed += r.rows_affected() as usize;
    }

    // An `articles` row with no remaining localizations is orphaned; drop it
    // (CASCADE handles FK dependents).
    sqlx::query(
        "DELETE FROM articles a WHERE a.repo_uri = $1 AND NOT EXISTS ( \
             SELECT 1 FROM article_localizations l \
             WHERE l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
         )",
    )
    .bind(repo_uri)
    .execute(&mut **tx)
    .await?;

    Ok(removed)
}

// ---- Tests ------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, content: &str) {
        let path = dir.join(rel);
        if let Some(p) = path.parent() {
            fs::create_dir_all(p).unwrap();
        }
        fs::write(&path, content).unwrap();
    }

    /// Smoke-test: walk a repo with MD source + translation, verify the
    /// collector returns two FileMeta rows.
    #[test]
    fn collect_basic() {
        let dir = TempDir::new().unwrap();
        write(
            dir.path(),
            "quantum.md",
            "---\nlang: en\ntitle: Quantum Intro\n---\nBody\n",
        );
        write(
            dir.path(),
            "quantum.zh-CN.md",
            "---\nlang: zh-CN\ntranslation-of: ./quantum.md\ntitle: 量子入门\n---\n内容\n",
        );

        let mut report = IndexReport::default();
        let metas = collect_file_metas(dir.path(), &mut report);
        assert_eq!(metas.len(), 2);
        assert!(report.errors.is_ok());
    }

    #[test]
    fn skips_cache_dirs() {
        let dir = TempDir::new().unwrap();
        write(
            dir.path(),
            "a.md",
            "---\nlang: en\n---\n",
        );
        write(
            dir.path(),
            "cache/b.md",
            "---\nlang: en\n---\n",
        );
        write(
            dir.path(),
            ".pijul/c.md",
            "---\nlang: en\n---\n",
        );

        let mut report = IndexReport::default();
        let metas = collect_file_metas(dir.path(), &mut report);
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].path, "a.md");
    }
}
