use std::sync::Arc;

use fx_atproto::client::AtClient;
use fx_core::region::InstanceMode;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// Scratch directory for PDS-blob-backed article working trees. Source files
    /// are materialized here on demand so the renderer sees a normal directory
    /// layout. Also used as the on-disk location for locally-authored content
    /// before it goes into a PDS blob. Set via FX_BLOB_CACHE_PATH.
    pub blob_cache_path: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
    pub at_client: AtClient,
    pub admin_secret: Option<String>,
    pub instance_mode: InstanceMode,
    pub session_store: Arc<dyn atproto_auth::SessionStore>,
    pub public_url: String,
    pub pds_url: String,
    pub orcid_client_id: Option<String>,
    pub orcid_client_secret: Option<String>,
}

// FromRef impls
impl axum::extract::FromRef<AppState> for Arc<dyn atproto_auth::SessionStore> {
    fn from_ref(state: &AppState) -> Self {
        state.session_store.clone()
    }
}

impl AppState {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let pool = fx_core::db::create_pool(&config.database_url).await?;
        let at_client = AtClient::new();
        let instance_mode = InstanceMode::from_str(&config.instance_mode);

        std::fs::create_dir_all(&config.blob_cache_path)?;
        let blob_cache_path = std::path::PathBuf::from(&config.blob_cache_path);

        // typst packages are shared across all repos, rooted as a sibling of
        // the blob cache so they don't get tangled with per-article state.
        let packages_dir = blob_cache_path
            .parent()
            .unwrap_or(&blob_cache_path)
            .join("typst-packages");
        std::fs::create_dir_all(&packages_dir)?;
        fx_renderer::set_packages_dir(packages_dir);

        let session_store: Arc<dyn atproto_auth::SessionStore> =
            Arc::new(atproto_auth::PgSessionStore::new(pool.clone()));

        tracing::info!("instance mode: {}", instance_mode.as_str());

        // User data directory: sibling of blob cache.
        let data_dir = blob_cache_path
            .parent()
            .unwrap_or(&blob_cache_path)
            .join("uploads");

        std::fs::create_dir_all(data_dir.join("book-covers"))?;
        std::fs::create_dir_all(data_dir.join("avatars"))?;
        std::fs::create_dir_all(data_dir.join("banners"))?;
        std::fs::create_dir_all(data_dir.join("covers"))?;

        Ok(Self {
            pool,
            blob_cache_path,
            data_dir,
            at_client,
            admin_secret: config.admin_secret.clone(),
            instance_mode,
            session_store,
            public_url: config.public_url.clone(),
            pds_url: config.pds_url.clone(),
            orcid_client_id: config.orcid_client_id.clone(),
            orcid_client_secret: config.orcid_client_secret.clone(),
        })
    }
}
