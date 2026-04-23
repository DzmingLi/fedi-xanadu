//! Repository layer (MVP).
//!
//! The traits here define a thin, mockable facade over the most common
//! per-aggregate operations. They let the service layer (and, later, HTTP
//! handlers) depend on a `dyn Repo` instead of `sqlx::PgPool + SQL strings`,
//! so we can:
//!
//!  * Swap backends (Redis cache, read-replica, in-memory mock for tests).
//!  * Batch or dataload across rows without rewriting every caller.
//!  * Introduce a `UnitOfWork` that spans pg + pijul + PDS once the trait
//!    surface stabilises.
//!
//! **Current scope is intentionally minimal.** This is not the repository
//! pattern taken to its logical extreme — it's a beachhead on the two
//! aggregates (`Article`, `Series`) that carry the most SQL duplication and
//! have the clearest identity (URI / id). Other aggregates stay in
//! `services/` for now; migrate them as the need becomes concrete.
//!
//! Each trait is paired with a `PgXxxRepo` default that delegates to the
//! existing `services/*_service.rs` helpers, so adoption is additive: a
//! handler can switch to `repo.owner(uri)` without breaking the old
//! `article_service::get_article_owner(&pool, uri)` call sites.

pub mod article;
pub mod series;

pub use article::{ArticleRepo, PgArticleRepo};
pub use series::{PgSeriesRepo, SeriesRepo};
