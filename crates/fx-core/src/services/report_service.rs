use sqlx::PgPool;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Report {
    pub id: String,
    pub reporter_did: String,
    pub target_did: String,
    pub target_uri: Option<String>,
    pub kind: String,
    pub reason: String,
    pub status: String,
    pub admin_note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ReportWithNames {
    pub id: String,
    pub reporter_did: String,
    pub reporter_handle: Option<String>,
    pub target_did: String,
    pub target_handle: Option<String>,
    pub target_uri: Option<String>,
    pub kind: String,
    pub reason: String,
    pub status: String,
    pub admin_note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn create_report(
    pool: &PgPool,
    id: &str,
    reporter_did: &str,
    target_did: &str,
    target_uri: Option<&str>,
    kind: &str,
    reason: &str,
) -> crate::Result<Report> {
    if reporter_did == target_did {
        return Err(crate::Error::BadRequest("cannot report yourself".into()));
    }

    let valid_kinds = ["user", "article", "comment"];
    if !valid_kinds.contains(&kind) {
        return Err(crate::Error::BadRequest(format!("invalid report kind: {kind}")));
    }

    sqlx::query(
        "INSERT INTO reports (id, reporter_did, target_did, target_uri, kind, reason) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(reporter_did)
    .bind(target_did)
    .bind(target_uri)
    .bind(kind)
    .bind(reason)
    .execute(pool)
    .await?;

    let report = sqlx::query_as::<_, Report>("SELECT * FROM reports WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;

    Ok(report)
}

/// List reports for admin review.
pub async fn list_reports(
    pool: &PgPool,
    status_filter: Option<&str>,
    limit: i64,
) -> crate::Result<Vec<ReportWithNames>> {
    let rows = if let Some(status) = status_filter {
        sqlx::query_as::<_, ReportWithNames>(
            "SELECT r.*, \
                 rp.handle AS reporter_handle, \
                 tp.handle AS target_handle \
             FROM reports r \
             LEFT JOIN profiles rp ON r.reporter_did = rp.did \
             LEFT JOIN profiles tp ON r.target_did = tp.did \
             WHERE r.status = $1 \
             ORDER BY r.created_at DESC LIMIT $2",
        )
        .bind(status)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ReportWithNames>(
            "SELECT r.*, \
                 rp.handle AS reporter_handle, \
                 tp.handle AS target_handle \
             FROM reports r \
             LEFT JOIN profiles rp ON r.reporter_did = rp.did \
             LEFT JOIN profiles tp ON r.target_did = tp.did \
             ORDER BY r.created_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    Ok(rows)
}

/// Resolve a report (admin action).
pub async fn resolve_report(
    pool: &PgPool,
    id: &str,
    status: &str,
    admin_note: Option<&str>,
) -> crate::Result<()> {
    let valid = ["resolved", "dismissed"];
    if !valid.contains(&status) {
        return Err(crate::Error::BadRequest(format!("invalid status: {status}")));
    }

    let result = sqlx::query(
        "UPDATE reports SET status = $1, admin_note = $2, resolved_at = NOW() WHERE id = $3",
    )
    .bind(status)
    .bind(admin_note)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound {
            entity: "report",
            id: id.to_string(),
        });
    }
    Ok(())
}
