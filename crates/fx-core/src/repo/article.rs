use async_trait::async_trait;
use sqlx::PgPool;

use crate::services::article_service;

/// Thin data-access facade over the `articles` table. Keep this trait small
/// on purpose: add a method only when a caller genuinely benefits from being
/// able to mock it or swap the backend. Don't forward every `article_service`
/// helper just because it exists.
#[async_trait]
pub trait ArticleRepo: Send + Sync {
    /// DID of the article's author. `Err(NotFound)` when the URI is unknown.
    async fn owner(&self, uri: &str) -> crate::Result<String>;

    /// Content format (`"markdown" | "typst" | "html"`). Same `NotFound`
    /// semantics as `owner`.
    async fn content_format(&self, uri: &str) -> crate::Result<String>;

    /// User's configured knot URL, or None (no override, fall back to the
    /// server-level default — handled one layer up in the service).
    async fn user_knot_url(&self, did: &str) -> Option<String>;
}

/// PostgreSQL-backed default. Delegates to the service functions so there's
/// exactly one implementation of each SQL query across the crate.
#[derive(Clone)]
pub struct PgArticleRepo {
    pool: PgPool,
}

impl PgArticleRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ArticleRepo for PgArticleRepo {
    async fn owner(&self, uri: &str) -> crate::Result<String> {
        article_service::get_article_owner(&self.pool, uri).await
    }

    async fn content_format(&self, uri: &str) -> crate::Result<String> {
        article_service::get_content_format(&self.pool, uri).await
    }

    async fn user_knot_url(&self, did: &str) -> Option<String> {
        article_service::get_user_knot_url(&self.pool, did).await
    }
}
