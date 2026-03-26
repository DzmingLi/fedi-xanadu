use sqlx::PgPool;

pub struct SearchEngine {
    pool: PgPool,
}

impl SearchEngine {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn search(&self, query: &str, limit: i64) -> anyhow::Result<Vec<String>> {
        let results = sqlx::query_scalar::<_, String>(
            "SELECT at_uri FROM articles
             WHERE search_vector @@ plainto_tsquery('simple', $1)
             ORDER BY ts_rank(search_vector, plainto_tsquery('simple', $1)) DESC
             LIMIT $2",
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }
}
