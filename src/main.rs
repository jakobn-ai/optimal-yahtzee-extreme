mod caching;
mod global;
mod hands;
mod repl;
mod rules;
mod strategy;
mod view_model;
mod yahtzee_bonus_rules;

use anyhow::{anyhow, ensure, Result};
use clap::{IntoApp, Parser};

#[derive(Parser)]
#[clap(about, version)]
struct Args {
    /// Use cache from <FILE>
    #[clap(long, value_name = "FILE")]
    cache: Option<String>,
    /// Pre-cache and write to <FILE>
    #[clap(long, value_name = "FILE")]
    cache_write: Option<String>,
    /// Game to play. Allowed options:{n}
    /// extreme  - Yahtzee Extreme{n}
    /// forced   - Forced choice joker, used in regular Yahtzee{n}
    /// free     - Free choice joker, a popular alternative{n}
    /// original - Original 1956 rules{n}
    /// kniffel  - Kniffel rules, as published in German-speaking countries{n}
    /// none     - No Yahtzee bonus
    game: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut app = Args::into_app();

    if let Some(filename) = args.cache_write {
        ensure!(args.cache.is_none(), "Caches cannot be used in pre-caching");
        caching::pre_cache(&filename)?;
        return Ok(());
    }

    if let Some(filename) = args.cache {
        caching::restore_caches(&filename)?;
    }

    if args.game.is_none() {
        println!("{}", app.render_usage());
        return Err(anyhow!("Must specify game to play unless pre-caching"));
    }
    let rules_result = build_rules(&args.game.unwrap());
    if let Err(e) = rules_result {
        println!("{}", app.render_usage());
        return Err(e);
    }
    let rules = rules_result.unwrap();
    let state = strategy::State::new_from_rules(&rules);
    Ok(repl::run(view_model::ViewModel {
        rules,
        state,
        rerolls: global::REROLLS,
    })?)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_rules() {
        assert_eq!(build_rules("extreme").unwrap().short_name, 'f');
        assert_eq!(build_rules("forced").unwrap().short_name, 'a');
        assert!(build_rules("null").is_err());
    }
}
