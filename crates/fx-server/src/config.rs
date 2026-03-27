use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub pijul_store_path: String,
    pub instance_name: String,
    /// Comma-separated string of allowed CORS origins. Empty = same-origin only.
    #[serde(default)]
    pub cors_origins: String,
    /// Shared secret for admin API endpoints. Set via FX_ADMIN_SECRET env var.
    #[serde(default)]
    pub admin_secret: Option<String>,
    /// Instance mode: "cn" or "intl" (default). Controls auth requirements
    /// and content visibility rules. Set via FX_INSTANCE_MODE env var.
    #[serde(default)]
    pub instance_mode: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3000,
            database_url: "postgres://localhost/fedi_xanadu".into(),
            pijul_store_path: "data/pijul-store".into(),
            instance_name: "Fedi-Xanadu".into(),
            cors_origins: String::new(),
            admin_secret: None,
            instance_mode: String::new(),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config: Config = Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file("fedi-xanadu.toml"))
            .merge(Env::prefixed("FX_"))
            .extract()?;
        Ok(config)
    }

    /// Parse cors_origins into a Vec of origin strings.
    pub fn cors_origin_list(&self) -> Vec<String> {
        if self.cors_origins.is_empty() {
            Vec::new()
        } else {
            self.cors_origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
    }
}
