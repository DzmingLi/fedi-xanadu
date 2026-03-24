use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

pub async fn create_pool(database_url: &str) -> crate::Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await?;

    init_schema(&pool).await?;
    run_migrations(&pool).await?;

    Ok(pool)
}

async fn init_schema(pool: &SqlitePool) -> crate::Result<()> {
    let schema = include_str!("../../../migrations/schema.sql");
    for statement in schema.split(';') {
        let trimmed = statement.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Err(e) = sqlx::query(trimmed).execute(pool).await {
            let msg = e.to_string();
            if msg.contains("already exists") {
                continue;
            }
            tracing::error!("schema init failed: {msg}\n  statement: {trimmed}");
            return Err(crate::Error::Internal(format!("schema init failed: {msg}")));
        }
    }
    Ok(())
}

/// Run numbered migration files (NNN_name.sql) that haven't been applied yet.
async fn run_migrations(pool: &SqlitePool) -> crate::Result<()> {
    // Create migrations tracking table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"
    )
    .execute(pool)
    .await?;

    // Collect migration files at compile time
    let migrations: &[(&str, &str)] = &[
        ("013", include_str!("../../../migrations/013_comments.sql")),
        ("014", include_str!("../../../migrations/014_skill_status.sql")),
        ("015", include_str!("../../../migrations/015_drafts.sql")),
        ("016", include_str!("../../../migrations/016_comment_replies_votes.sql")),
    ];

    for (num, sql) in migrations {
        let id: i64 = num.parse().unwrap();
        let already: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM _migrations WHERE id = ?)"
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        if already {
            continue;
        }

        tracing::info!("applying migration {num}");
        for statement in sql.split(';') {
            let trimmed = statement.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Err(e) = sqlx::query(trimmed).execute(pool).await {
                let msg = e.to_string();
                // Skip "duplicate column" etc. for idempotent migrations
                if msg.contains("duplicate column") || msg.contains("already exists") {
                    continue;
                }
                tracing::error!("migration {num} failed: {msg}\n  statement: {trimmed}");
                return Err(crate::Error::Internal(format!("migration {num} failed: {msg}")));
            }
        }

        sqlx::query("INSERT INTO _migrations (id, name) VALUES (?, ?)")
            .bind(id)
            .bind(format!("{num}"))
            .execute(pool)
            .await?;
    }

    Ok(())
}
