use pijul_config::{Choice, Config};
use std::path::PathBuf;

const GLOBAL_CONFIG: &str = r#"
colors = "never"
pager = "never"
reset_overwrites_changes = "never"
"#;

const LOCAL_CONFIG: &str = r#"
colors = "always"
pager = "always"
reset_overwrites_changes = "always"
"#;

const CONFIG_OVERRIDES: [(&str, &str); 3] = [
    ("colors", "auto"),
    ("pager", "auto"),
    ("reset_overwrites_changes", "auto"),
];

fn check_config_fields(config: &Config, choice: Choice) {
    assert_eq!(config.colors, choice);
    assert_eq!(config.pager, choice);
    assert_eq!(config.reset_overwrites_changes, choice);
}

/// Default config values should be Choice::Auto
#[test]
fn default() -> Result<(), anyhow::Error> {
    let default_config = Config::load(None, Vec::new())?;

    // Double-check that the defaults still use Choice::Auto
    check_config_fields(&default_config, Choice::Auto);

    Ok(())
}

/// Global config values should override defaults
#[test]
fn global() -> Result<(), anyhow::Error> {
    let global_config = Config::load_with(
        Some((PathBuf::new(), String::from(GLOBAL_CONFIG))),
        None,
        Vec::new(),
    )?;

    // Make sure the global config overrides these fields
    check_config_fields(&global_config, Choice::Never);

    Ok(())
}

/// Local config values should override defaults and global config
#[test]
fn local() -> Result<(), anyhow::Error> {
    let local_config = Config::load_with(
        Some((PathBuf::new(), String::from(GLOBAL_CONFIG))),
        Some((PathBuf::new(), String::from(LOCAL_CONFIG))),
        Vec::new(),
    )?;

    // Make sure the local config overrides these fields
    check_config_fields(&local_config, Choice::Always);

    Ok(())
}

/// Config overrides should override everything
#[test]
fn overrides() -> Result<(), anyhow::Error> {
    let override_config = Config::load_with(
        Some((PathBuf::new(), String::from(GLOBAL_CONFIG))),
        Some((PathBuf::new(), String::from(LOCAL_CONFIG))),
        CONFIG_OVERRIDES
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
    )?;

    // Make sure the overrides apply to these fields
    check_config_fields(&override_config, Choice::Auto);

    Ok(())
}

/// Different layers merge correctly
#[test]
fn merging() -> Result<(), anyhow::Error> {
    let layered_config = Config::load_with(
        Some((PathBuf::new(), String::from(r#"colors = "never""#))),
        Some((PathBuf::new(), String::from(r#"pager = "always""#))),
        vec![(
            String::from("reset_overwrites_changes"),
            String::from("never"),
        )],
    )?;

    // Make sure the layers merge correctly
    assert_eq!(layered_config.colors, Choice::Never);
    assert_eq!(layered_config.pager, Choice::Always);
    assert_eq!(layered_config.reset_overwrites_changes, Choice::Never);

    Ok(())
}
