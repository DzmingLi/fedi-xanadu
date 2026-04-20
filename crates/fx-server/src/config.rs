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
    /// Public URL of this instance (for OAuth client_id and callback).
    /// e.g. "https://xanadu.example.com". Set via FX_PUBLIC_URL env var.
    #[serde(default)]
    pub public_url: String,
    /// Default knot (pijul hosting) URL used when a user has not configured
    /// their own `user_settings.knot_url`. Embedded into at.nightbo.article
    /// and at.nightbo.series records so external AppViews can clone the source.
    /// Set via FX_DEFAULT_KNOT_URL. e.g. "https://knot.dzming.li".
    #[serde(default)]
    pub default_knot_url: String,
    /// URL of the ATProto PDS this instance treats as its home PDS — new
    /// signups are created there and password logins proxy to its
    /// `com.atproto.server.createSession`. Empty means register / local
    /// password login are disabled. Set via FX_PDS_URL.
    #[serde(default)]
    pub pds_url: String,
    /// ORCID OAuth client ID. Register at https://orcid.org/developer-tools
    #[serde(default)]
    pub orcid_client_id: Option<String>,
    /// ORCID OAuth client secret.
    #[serde(default)]
    pub orcid_client_secret: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3000,
            database_url: "postgres://localhost/nightboat".into(),
            pijul_store_path: "data/pijul-store".into(),
            instance_name: "NightBoat".into(),
            cors_origins: String::new(),
            admin_secret: None,
            instance_mode: String::new(),
            public_url: String::new(),
            default_knot_url: String::new(),
            pds_url: String::new(),
            orcid_client_id: None,
            orcid_client_secret: None,
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config: Config = Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file("nightboat.toml"))
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
