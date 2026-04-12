use std::sync::Arc;

use fx_atproto::client::AtClient;
use fx_core::region::InstanceMode;
use pijul_knot::PijulStore;
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
}

// Allow atproto-auth's AuthUser extractor to pull SessionStore from AppState.
impl axum::extract::FromRef<AppState> for Arc<dyn atproto_auth::SessionStore> {
    fn from_ref(state: &AppState) -> Self {
        state.session_store.clone()
    }
}

impl AppState {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let pool = fx_core::db::create_pool(&config.database_url).await?;
        let pijul = Arc::new(PijulStore::new(&config.pijul_store_path));
        let at_client = AtClient::new();
        let instance_mode = InstanceMode::from_str(&config.instance_mode);

        std::fs::create_dir_all(&config.pijul_store_path)?;

        // Initialize Typst package cache directory
        let packages_dir = std::path::PathBuf::from(&config.pijul_store_path).join("typst-packages");
        std::fs::create_dir_all(&packages_dir)?;
        fx_render::set_packages_dir(packages_dir);

        let session_store: Arc<dyn atproto_auth::SessionStore> =
            Arc::new(atproto_auth::PgSessionStore::new(pool.clone()));

        tracing::info!("instance mode: {}", instance_mode.as_str());

        Ok(Self {
            pool,
            pijul,
            at_client,
            admin_secret: config.admin_secret.clone(),
            instance_mode,
            session_store,
        })
    }
}
