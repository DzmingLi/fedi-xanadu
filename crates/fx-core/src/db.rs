use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> crate::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await?;

    // Set statement timeout to prevent runaway queries
    sqlx::query("SET statement_timeout = '30s'")
        .execute(&pool)
        .await?;

    sqlx::migrate!("../../migrations_pg")
        .run(&pool)
        .await
        .map_err(|e| crate::Error::Internal(format!("migration failed: {e}")))?;

    Ok(pool)
}
