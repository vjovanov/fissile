//! The checked-in config fixtures must load under the real schema
//! (§FS-001-config, §DF-002-explicit-config).

use std::fs;

use fissile::config::Config;

fn load(relative: &str) -> Config {
    let path = format!("{}/{relative}", env!("CARGO_MANIFEST_DIR"));
    let text = fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path}: {error}"));
    Config::parse(&text).unwrap_or_else(|error| panic!("parse {relative}: {error}"))
}

#[test]
fn repo_config_loads_and_builds() {
    let config = load(".agents/fissile.toml");
    config
        .to_checker()
        .expect(".agents/fissile.toml builds a checker");
}

#[test]
fn example_config_loads_and_builds() {
    let config = load("examples/fissile.toml");
    config
        .to_checker()
        .expect("examples/fissile.toml builds a checker");
}
