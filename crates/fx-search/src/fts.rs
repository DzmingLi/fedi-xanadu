use sqlx::SqlitePool;

pub struct SearchEngine {
    pool: SqlitePool,
}

impl SearchEngine {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn search(&self, query: &str, limit: i64) -> anyhow::Result<Vec<String>> {
        let results = sqlx::query_scalar::<_, String>(
            "SELECT at_uri FROM articles WHERE at_uri IN (
                SELECT rowid FROM articles_fts WHERE articles_fts MATCH ?1
            ) LIMIT ?2",
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }
}
