use sqlx::PgPool;

/// Hybrid search: tsvector for English word-based + pg_trgm for CJK/fuzzy matching.
pub async fn search_articles(pool: &PgPool, query: &str, limit: i64) -> anyhow::Result<Vec<String>> {
    let results = sqlx::query_scalar::<_, String>(
        "SELECT l.at_uri FROM articles a
         JOIN article_localizations l
           ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path
         WHERE l.at_uri IS NOT NULL
           AND l.file_path = a.source_path
           AND (l.search_vector @@ plainto_tsquery('simple', $1)
             OR similarity(l.title, $1) > 0.1
             OR similarity(l.summary, $1) > 0.1)
         ORDER BY GREATEST(
           ts_rank(l.search_vector, plainto_tsquery('simple', $1)),
           similarity(l.title, $1) * 2.0,
           similarity(l.summary, $1)
         ) DESC
         LIMIT $2",
    )
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(results)
}
