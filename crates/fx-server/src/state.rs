use std::sync::Arc;

use fx_atproto::client::AtClient;
use fx_pijul::PijulStore;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub pijul: Arc<PijulStore>,
    pub at_client: AtClient,
    pub admin_secret: Option<String>,
}

impl AppState {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let pool = fx_core::db::create_pool(&config.database_url).await?;
        let pijul = Arc::new(PijulStore::new(&config.pijul_store_path));
        let at_client = AtClient::new();

        std::fs::create_dir_all(&config.pijul_store_path)?;

        Ok(Self {
            pool,
            pijul,
            at_client,
            admin_secret: config.admin_secret.clone(),
        })
    }
}
