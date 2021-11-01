// to quiet warnings, TODO use/clean up
#![allow(dead_code)]

use std::iter::repeat;

use crate::global::*;
use crate::rules;
use crate::strategy;
use crate::yahtzee_bonus_rules as bonus;

/// Populate all caches by (transitively) calling all cachable functions with their entire domains
fn populate_caches() {
    repeat(false)
        .zip(bonus::ALL_VARIANTS.iter().cloned())
        .chain([(true, bonus::NONE)].iter().cloned())
        .for_each(|(extreme, yahtzee_bonus)| {
            let rules = rules::build_rules(extreme, yahtzee_bonus);
            let state = strategy::State {
                score: [0, 0],
                used: [
                    vec![false; rules.fields[0].len()],
                    vec![false; rules.fields[1].len()],
                ],
                scored_yahtzee: false,
                chips: rules.chips,
            };
            strategy::choose_reroll(state, vec![], THROWS, &rules);
        });
}
