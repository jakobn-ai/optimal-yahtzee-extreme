mod caching;
mod global;
mod hands;
mod repl;
mod rules;
mod strategy;
mod view_model;
mod yahtzee_bonus_rules;

use anyhow::{anyhow, ensure, Result};

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
        ensure!(cache.is_none(), "Caches cannot be used in pre-caching");
        caching::pre_cache(filename)?;
        return Ok(());
    }

    if let Some(filename) = cache {
        caching::restore_caches(filename)?;
    }

    let game = matches.value_of("game");
    if game.is_none() {
        println!("{}", matches.usage());
        return Err(anyhow!("Must specify game to play unless pre-caching"));
    }
    let rules_result = build_rules(game.unwrap());
    if let Err(e) = rules_result {
        println!("{}", matches.usage());
        return Err(e);
    }
    let rules = rules_result.unwrap();
    let state = strategy::State::new_from_rules(&rules);
    Ok(repl::run(view_model::ViewModel {
        rules,
        state,
        rerolls: global::THROWS - 1,
    })?)
}

// TODO test
fn build_rules(game: &str) -> Result<rules::Rules> {
    if game == "extreme" {
        return Ok(rules::build_rules(true, yahtzee_bonus_rules::NONE));
    }
    yahtzee_bonus_rules::ALL_VARIANTS_NAMES
        .iter()
        .enumerate()
        .find(|(_, &name)| name == game)
        .map(|(i, _)| rules::build_rules(false, yahtzee_bonus_rules::ALL_VARIANTS[i].clone()))
        .ok_or_else(|| anyhow!("Unknown game: {}", game))
}
