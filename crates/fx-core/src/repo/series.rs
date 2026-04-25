use async_trait::async_trait;
use sqlx::PgPool;

use crate::services::series_service;

/// Thin data-access facade over the `series` table. See `repo::article` for
/// the rationale on keeping the method set small.
#[async_trait]
pub trait SeriesRepo: Send + Sync {
    /// DID of the series creator. `Err(NotFound)` when the id is unknown.
    async fn owner(&self, id: &str) -> crate::Result<String>;
}

#[derive(Clone)]
pub struct PgSeriesRepo {
    pool: PgPool,
}

impl PgSeriesRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SeriesRepo for PgSeriesRepo {
    async fn owner(&self, id: &str) -> crate::Result<String> {
        series_service::get_series_owner(&self.pool, id).await
    }
}
