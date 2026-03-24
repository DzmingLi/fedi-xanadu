use pijul_config::parse_config_arg;

#[test]
fn top_level() -> Result<(), anyhow::Error> {
    let (key, value) = parse_config_arg("unrecord_changes=1")?;

    assert_eq!(key, "unrecord_changes");
    assert_eq!(value, "1");

    Ok(())
}

#[test]
fn nested() -> Result<(), anyhow::Error> {
    let (key, value) = parse_config_arg("author.username=Ferris")?;

    assert_eq!(key, "author.username");
    assert_eq!(value, "Ferris");

    Ok(())
}

#[test]
fn missing_equals_sign() {
    parse_config_arg("unrecord_changes1").unwrap_err();
}

#[test]
fn empty_argument() {
    parse_config_arg("").unwrap_err();
}
