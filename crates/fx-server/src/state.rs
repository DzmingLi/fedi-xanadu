use std::sync::Arc;

use fx_atproto::client::AtClient;
use fx_core::region::InstanceMode;
use pijul_knot::{PadProjectResolver, PadError, PijulStore};
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub pijul: Arc<PijulStore>,
    pub at_client: AtClient,
    pub admin_secret: Option<String>,
    pub instance_mode: InstanceMode,
    pub session_store: Arc<dyn atproto_auth::SessionStore>,
    pub series_resolver: Arc<dyn PadProjectResolver>,
}

// FromRef impls
impl axum::extract::FromRef<AppState> for Arc<dyn atproto_auth::SessionStore> {
    fn from_ref(state: &AppState) -> Self {
        state.session_store.clone()
    }
}

impl axum::extract::FromRef<AppState> for Arc<PijulStore> {
    fn from_ref(state: &AppState) -> Self {
        state.pijul.clone()
    }
}

impl axum::extract::FromRef<AppState> for Arc<dyn PadProjectResolver> {
    fn from_ref(state: &AppState) -> Self {
        state.series_resolver.clone()
    }
}

impl AppState {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let pool = fx_core::db::create_pool(&config.database_url).await?;
        let pijul = Arc::new(PijulStore::new(&config.pijul_store_path));
        let at_client = AtClient::new();
        let instance_mode = InstanceMode::from_str(&config.instance_mode);

        std::fs::create_dir_all(&config.pijul_store_path)?;

        let packages_dir = std::path::PathBuf::from(&config.pijul_store_path).join("typst-packages");
        std::fs::create_dir_all(&packages_dir)?;
        fx_renderer::set_packages_dir(packages_dir);

        let session_store: Arc<dyn atproto_auth::SessionStore> =
            Arc::new(atproto_auth::PgSessionStore::new(pool.clone()));

        let series_resolver: Arc<dyn PadProjectResolver> =
            Arc::new(PgSeriesResolver { pool: pool.clone() });

        tracing::info!("instance mode: {}", instance_mode.as_str());

        Ok(Self {
            pool,
            pijul,
            at_client,
            admin_secret: config.admin_secret.clone(),
            instance_mode,
            session_store,
            series_resolver,
        })
    }
}

/// PadProjectResolver for nightboat series — resolves series ID to pijul node_id.
struct PgSeriesResolver {
    pool: PgPool,
}

#[async_trait::async_trait]
impl PadProjectResolver for PgSeriesResolver {
    async fn resolve_node_id(&self, series_id: &str) -> Result<String, PadError> {
        let row: Option<Option<String>> = sqlx::query_scalar("SELECT pijul_node_id FROM series WHERE id = $1")
            .bind(series_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| PadError::Internal(e.to_string()))?;
        row.flatten()
            .ok_or(PadError::NotFound("series not found or no pijul repo".into()))
    }

    async fn get_knot_url(&self, series_id: &str) -> Option<String> {
        // Look up the series author's knot_url from user_settings
        let author_did: Option<String> = sqlx::query_scalar("SELECT created_by FROM series WHERE id = $1")
            .bind(series_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();
        if let Some(did) = author_did {
            sqlx::query_scalar::<_, Option<String>>("SELECT knot_url FROM user_settings WHERE did = $1")
                .bind(&did)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten()
                .flatten()
                .filter(|u| !u.is_empty())
        } else {
            None
        }
    }

    async fn get_owner_did(&self, series_id: &str) -> Result<String, PadError> {
        sqlx::query_scalar::<_, String>("SELECT author_did FROM series WHERE id = $1")
            .bind(series_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| PadError::Internal(e.to_string()))?
            .ok_or(PadError::NotFound("series not found".into()))
    }

    async fn on_record(&self, series_id: &str) {
        let _ = sqlx::query("UPDATE series SET updated_at = NOW() WHERE id = $1")
            .bind(series_id)
            .execute(&self.pool)
            .await;
    }
}
