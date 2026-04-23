//! Cross-store consistency reconciliation for the three persistence layers:
//! PostgreSQL rows, pijul on-disk repos, and (future) AT Protocol PDS records.
//!
//! MVP covers only the pg ↔ pijul axis because that's where most bugs have
//! hit so far (nsid renames leaving stale dirs, half-finished creates with
//! pijul repos but no DB row). PDS reconciliation is expensive — it requires
//! an HTTP round-trip per author PDS — and is deferred until needed.
//!
//! This module is **read-only**. It reports inconsistencies; callers decide
//! what to do. Auto-deletion of orphans is intentionally not offered: the
//! orphan dir might be uncommitted user work that hasn't landed in the DB
//! yet, and nuking it unprompted would be the worst possible regression.

use std::collections::HashSet;
use std::path::Path;

use sqlx::PgPool;

use crate::util::uri_to_node_id;

/// One half-broken reference. A stable pair of (kind, node_id) — the
/// PostgreSQL side identifies the row, the filesystem side says whether the
/// repo exists.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Mismatch {
    pub kind: &'static str,
    pub node_id: String,
    /// For articles, the `at_uri` that generated this node_id. For series,
    /// the series.id. Empty when the entry came from filesystem scan only.
    pub ref_id: String,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ConsistencyReport {
    /// Articles whose content_storage = 'pijul' but whose repo dir is missing.
    pub article_missing_repo: Vec<Mismatch>,
    /// Series with pijul_node_id set but no corresponding repo on disk.
    pub series_missing_repo: Vec<Mismatch>,
    /// Repo directories on disk that don't map to any current DB row.
    /// Often legitimate (forks-in-progress, deleted articles kept for
    /// appeals, etc.), but worth surfacing so ops can investigate.
    pub orphan_dirs: Vec<Mismatch>,
    /// Total repo dirs scanned. Used for ballpark sanity checks.
    pub total_dirs_scanned: usize,
}

/// Run one reconciliation pass. Safe to call concurrently with writes —
/// consistency reports are best-effort snapshots, never blocking.
pub async fn check_pijul(pool: &PgPool, pijul_store: &Path) -> crate::Result<ConsistencyReport> {
    let mut report = ConsistencyReport::default();

    // --- Articles ---
    // Only articles whose bytes live in pijul. Blob-backed articles resolve
    // via blob_cache_path and are out of scope for this pass.
    let article_rows: Vec<(String,)> = sqlx::query_as(
        "SELECT at_uri FROM articles \
         WHERE content_storage IS NULL OR content_storage = 'pijul'",
    )
    .fetch_all(pool)
    .await?;

    let mut expected_dirs: HashSet<String> = HashSet::new();
    for (uri,) in &article_rows {
        let node_id = uri_to_node_id(uri);
        if !pijul_store.join(&node_id).is_dir() {
            report.article_missing_repo.push(Mismatch {
                kind: "article",
                node_id: node_id.clone(),
                ref_id: uri.clone(),
            });
        }
        expected_dirs.insert(node_id);
    }

    // --- Series ---
    let series_rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, pijul_node_id FROM series \
         WHERE pijul_node_id IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    for (series_id, node_id) in &series_rows {
        if !pijul_store.join(node_id).is_dir() {
            report.series_missing_repo.push(Mismatch {
                kind: "series",
                node_id: node_id.clone(),
                ref_id: series_id.clone(),
            });
        }
        expected_dirs.insert(node_id.clone());
    }

    // --- Orphan dirs on disk ---
    // Skip the handful of well-known system subdirectories the pijul store
    // uses for its own bookkeeping (typst packages, scratch spaces, etc.).
    const SYSTEM_DIRS: &[&str] = &["typst-packages", ".tmp", ".cache"];

    if let Ok(entries) = std::fs::read_dir(pijul_store) {
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if !ft.is_dir() { continue; }
            let name = entry.file_name().to_string_lossy().to_string();
            if SYSTEM_DIRS.contains(&name.as_str()) { continue; }
            if name.starts_with('.') { continue; }
            report.total_dirs_scanned += 1;
            if !expected_dirs.contains(&name) {
                report.orphan_dirs.push(Mismatch {
                    kind: if name.starts_with("series_") { "series" } else { "article" },
                    node_id: name,
                    ref_id: String::new(),
                });
            }
        }
    }

    Ok(report)
}
