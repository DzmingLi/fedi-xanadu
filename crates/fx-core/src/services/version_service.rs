use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

// --- Types ---

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ArticleVersion {
    pub id: i32,
    pub article_uri: String,
    pub change_hash: String,
    pub editor_did: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

/// A single version with full source text (for viewing old versions).
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ArticleVersionFull {
    pub id: i32,
    pub article_uri: String,
    pub change_hash: String,
    pub editor_did: String,
    pub message: String,
    pub source_text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct VersionDiff {
    pub from_version: i32,
    pub to_version: i32,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct DiffLine {
    /// "context", "add", or "remove"
    pub kind: String,
    pub content: String,
}

// --- Service functions ---

pub async fn record_version(
    pool: &PgPool,
    article_uri: &str,
    change_hash: &str,
    editor_did: &str,
    message: &str,
    source_text: &str,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO article_versions (article_uri, change_hash, editor_did, message, source_text)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(article_uri)
    .bind(change_hash)
    .bind(editor_did)
    .bind(message)
    .bind(source_text)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_versions(
    pool: &PgPool,
    article_uri: &str,
) -> crate::Result<Vec<ArticleVersion>> {
    let versions = sqlx::query_as::<_, ArticleVersion>(
        "SELECT id, article_uri, change_hash, editor_did, message, created_at
         FROM article_versions
         WHERE article_uri = $1
         ORDER BY created_at ASC",
    )
    .bind(article_uri)
    .fetch_all(pool)
    .await?;
    Ok(versions)
}

pub async fn get_version(
    pool: &PgPool,
    article_uri: &str,
    version_id: i32,
) -> crate::Result<ArticleVersionFull> {
    let version = sqlx::query_as::<_, ArticleVersionFull>(
        "SELECT id, article_uri, change_hash, editor_did, message, source_text, created_at
         FROM article_versions
         WHERE article_uri = $1 AND id = $2",
    )
    .bind(article_uri)
    .bind(version_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound {
        entity: "version",
        id: version_id.to_string(),
    })?;
    Ok(version)
}

pub async fn diff_versions(
    pool: &PgPool,
    article_uri: &str,
    from_id: i32,
    to_id: i32,
) -> crate::Result<VersionDiff> {
    let from = get_version(pool, article_uri, from_id).await?;
    let to = get_version(pool, article_uri, to_id).await?;

    let hunks = compute_diff(&from.source_text, &to.source_text);

    Ok(VersionDiff {
        from_version: from_id,
        to_version: to_id,
        hunks,
    })
}

/// Compute unified diff hunks between two texts with 3 lines of context.
fn compute_diff(old: &str, new: &str) -> Vec<DiffHunk> {
    use imara_diff::intern::InternedInput;
    use imara_diff::{diff, Algorithm, UnifiedDiffBuilder};

    let input = InternedInput::new(old, new);
    let diff_output = diff(
        Algorithm::Histogram,
        &input,
        UnifiedDiffBuilder::new(&input),
    );

    // Parse the unified diff output into structured hunks
    parse_unified_diff(&diff_output)
}

/// Parse unified diff text into structured DiffHunks.
fn parse_unified_diff(diff_text: &str) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<DiffHunk> = None;

    for line in diff_text.lines() {
        if line.starts_with("@@") {
            // Flush previous hunk
            if let Some(h) = current_hunk.take() {
                hunks.push(h);
            }
            // Parse @@ -old_start,old_count +new_start,new_count @@
            if let Some(h) = parse_hunk_header(line) {
                current_hunk = Some(h);
            }
        } else if let Some(ref mut hunk) = current_hunk {
            if let Some(content) = line.strip_prefix('+') {
                hunk.lines.push(DiffLine {
                    kind: "add".to_string(),
                    content: content.to_string(),
                });
            } else if let Some(content) = line.strip_prefix('-') {
                hunk.lines.push(DiffLine {
                    kind: "remove".to_string(),
                    content: content.to_string(),
                });
            } else if let Some(content) = line.strip_prefix(' ') {
                hunk.lines.push(DiffLine {
                    kind: "context".to_string(),
                    content: content.to_string(),
                });
            } else {
                hunk.lines.push(DiffLine {
                    kind: "context".to_string(),
                    content: line.to_string(),
                });
            }
        }
    }

    if let Some(h) = current_hunk {
        hunks.push(h);
    }

    hunks
}

fn parse_hunk_header(line: &str) -> Option<DiffHunk> {
    // @@ -1,3 +1,4 @@
    let line = line.trim_start_matches("@@ ").trim_end_matches(" @@");
    let parts: Vec<&str> = line.split(' ').collect();
    if parts.len() < 2 {
        return None;
    }

    let old = parts[0].trim_start_matches('-');
    let new = parts[1].trim_start_matches('+');

    let (old_start, old_count) = parse_range(old);
    let (new_start, new_count) = parse_range(new);

    Some(DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        lines: Vec::new(),
    })
}

pub async fn delete_version(pool: &PgPool, version_id: i32) -> crate::Result<()> {
    sqlx::query("DELETE FROM article_versions WHERE id = $1")
        .bind(version_id)
        .execute(pool)
        .await?;
    Ok(())
}

fn parse_range(s: &str) -> (usize, usize) {
    if let Some((start, count)) = s.split_once(',') {
        (
            start.parse().unwrap_or(1),
            count.parse().unwrap_or(1),
        )
    } else {
        (s.parse().unwrap_or(1), 1)
    }
}
