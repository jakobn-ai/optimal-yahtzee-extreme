mod caching;
mod global;
mod hands;
mod rules;
mod strategy;
mod view_model;
mod yahtzee_bonus_rules;

use anyhow::{ensure, Result};

#[macro_use]
extern crate clap;
use clap::App;

fn main() -> Result<()> {
    let cli_yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(cli_yaml)
        .version(crate_version!())
        .get_matches();
    let cache = matches.value_of("cache");

    if let Some(filename) = matches.value_of("cache-write") {
        ensure!(cache.is_none(), "caches cannot be used in pre-caching");
        caching::pre_cache(filename)?;
    }
    if let Some(filename) = cache {
        caching::restore_caches(filename)?;
    }

    Ok(())
    // TODO REPL
}
