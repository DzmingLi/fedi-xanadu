use sqlx::PgPool;

pub struct SearchEngine {
    pool: PgPool,
}

impl SearchEngine {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Hybrid search: tsvector for English word-based + pg_trgm for CJK/fuzzy matching.
    pub async fn search(&self, query: &str, limit: i64) -> anyhow::Result<Vec<String>> {
        let results = sqlx::query_scalar::<_, String>(
            "SELECT at_uri FROM articles
             WHERE search_vector @@ plainto_tsquery('simple', $1)
                OR similarity(title, $1) > 0.1
                OR similarity(description, $1) > 0.1
             ORDER BY GREATEST(
               ts_rank(search_vector, plainto_tsquery('simple', $1)),
               similarity(title, $1) * 2.0,
               similarity(description, $1)
             ) DESC
             LIMIT $2",
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }
}
