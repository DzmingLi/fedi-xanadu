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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3000,
            database_url: "sqlite://data/fedi-xanadu.db?mode=rwc".into(),
            pijul_store_path: "data/pijul-store".into(),
            instance_name: "Fedi-Xanadu".into(),
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
}
