use pijul_config::Config;
use std::path::PathBuf;

#[test]
fn load_simple() -> Result<(), anyhow::Error> {
    Config::load(None, Vec::new())?;

    Ok(())
}

#[test]
fn load_defaults() -> Result<(), anyhow::Error> {
    Config::load_with(None, None, Vec::new())?;

    Ok(())
}

#[test]
fn empty_global_config() -> Result<(), anyhow::Error> {
    let empty_config = Some((PathBuf::new(), String::new()));
    Config::load_with(empty_config, None, Vec::new())?;

    Ok(())
}

#[test]
fn empty_local_config() -> Result<(), anyhow::Error> {
    let empty_config = Some((PathBuf::new(), String::new()));
    Config::load_with(None, empty_config, Vec::new())?;

    Ok(())
}
