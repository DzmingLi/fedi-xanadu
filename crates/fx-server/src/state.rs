use std::sync::Arc;

use fx_atproto::client::AtClient;
use fx_pijul::PijulStore;
use sqlx::SqlitePool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub pijul: Arc<PijulStore>,
    pub config: Config,
    pub at_client: AtClient,
}

impl AppState {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        // Ensure data directory exists
        if let Some(parent) = std::path::Path::new(&config.database_url)
            .to_str()
            .and_then(|s| s.strip_prefix("sqlite://"))
            .and_then(|s| s.split('?').next())
            .and_then(|s| std::path::Path::new(s).parent())
        {
            std::fs::create_dir_all(parent)?;
        }

        let pool = fx_core::db::create_pool(&config.database_url).await?;
        let pijul = Arc::new(PijulStore::new(&config.pijul_store_path));
        let at_client = AtClient::new();

        std::fs::create_dir_all(&config.pijul_store_path)?;

        Ok(Self {
            pool,
            pijul,
            config: config.clone(),
            at_client,
        })
    }
}
