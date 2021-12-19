use crate::global::*;
use crate::rules;
use crate::yahtzee_bonus_rules as bonus;

use std::collections::HashMap;

use cached::proc_macro::cached;
use float_cmp::approx_eq;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[cfg(target_pointer_width = "64")]
type ArchFloat = f64;
#[cfg(target_pointer_width = "32")]
type ArchFloat = f32;
/// Expectation value
type Expectation = ArchFloat;

/// Statistical probability
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Probability(ArchFloat);

// For the tests, only using `approx_eq!` for these probabilities (but not expectation values)
// Works On My Machine(tm), but similar implementations for expectation values might be required.
impl PartialEq for Probability {
    fn eq(&self, other: &Self) -> bool {
        approx_eq!(ArchFloat, self.0, other.0)
    }
}

/// State with everything relevant to strategy
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub score: [Score; 2],
    pub used: ScoreCard,
    pub scored_yahtzee: bool,
    pub chips: Chips,
}

impl Clone for State {
    fn clone(&self) -> Self {
        State {
            score: self.score,
            used: self.used.clone(),
            scored_yahtzee: self.scored_yahtzee,
            chips: self.chips,
        }
    }
}

impl State {
    /// Build initial state from rules
    pub fn new_from_rules(rules: &rules::Rules) -> Self {
        State {
            score: [0, 0],
            used: [
                [false].repeat(rules.fields[0].len()),
                [false].repeat(rules.fields[1].len()),
            ],
            scored_yahtzee: false,
            chips: rules.chips,
        }
    }

    /// Compact format for cache keys
    pub fn compact_fmt(&self) -> String {
        format!(
            "{},{}{},{}",
            format!("{},{}", self.score[0], self.score[1]),
            self.used
                .iter()
                .map(|section| section
                    .iter()
                    .map(|&field| field as i8)
                    .fold(String::from(""), |a, b| format!("{}{}", a, b)))
                .reduce(|a, b| format!("{},{}", a, b))
                .unwrap(),
            self.scored_yahtzee as i8,
            self.chips,
        )
    }
}

/// Hash map of all reachable hands and probabilities
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbabilitiesToRoll {
    /// Wrapped in struct for serializability
    #[serde_as(as = "Vec<(_, _)>")]
    pub table: HashMap<PartialHand, Probability>,
}

/// Recommendation for what to keep for rerolling
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RerollRecomm {
    /// Hand to keep
    pub hand: PartialHand,
    /// State - passed on unchanged
    pub state: State,
    /// Expectation value when keeping this hand
    pub expectation: Expectation,
}

/// Recommendation for which field to use for score
// TODO should indicate whether this was a bonus
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldRecomm {
    /// Section to choose
    pub section: Section,
    /// Field to choose
    pub field: Field,
    /// State after choosing this field
    pub state: State,
    /// Expectation value when choosing this field
    pub expectation: Expectation,
}

impl Clone for FieldRecomm {
    fn clone(&self) -> Self {
        FieldRecomm {
            section: self.section,
            field: self.field,
            state: self.state.clone(),
            expectation: self.expectation,
        }
    }
}

/// Probability to roll hands given hand
/// # Arguments
/// * `have` - partial hand to start with
/// * `rules` - dice rules
/// # Returns
/// Probabilities to roll - see architecture of structure above, hands sorted
#[cached(
    key = "String",
    convert = r#"{ format!("{}{}", have.compact_fmt(), rules.short_name ) }"#
)]
pub fn probability_to_roll(have: PartialHand, rules: &rules::DiceRules) -> ProbabilitiesToRoll {
    // Calculate dice left to use
    let mut leftover = rules.dice.0.to_owned();
    'next_have: for &(have_die, _) in &have.0 {
        for (left_die, freq) in &mut leftover {
            if have_die == *left_die {
                *freq -= 1;
                continue 'next_have;
            }
        }
        panic!("Mismatch between hand and rules");
    }

    // Calculate all possible hands
    let mut hands = vec![have];
    for &((min, max), frequency) in &leftover {
        for _ in 0..frequency {
            hands = hands
                .iter()
                .flat_map(|hand| {
                    (min..(max + 1)).map(move |pip| {
                        // Append possible pip to hand
                        let mut new_hand = hand.clone();
                        new_hand.0.push(((min, max), pip));
                        new_hand
                    })
                })
                .collect();
        }
    }

    // Calculate total possible hands by multiplication
    let total = leftover.iter().fold(1, |total, ((min, max), frequency)| {
        total * i32::pow((max - min + 1) as i32, *frequency as u32)
    });
    let probability_per_hand = 1.0 / total as ArchFloat;

    // Sort hands and add up probabilities
    let mut probabilities = HashMap::new();
    for mut hand in hands {
        hand.0.sort_unstable_by_key(|&(_, pip)| pip);
        probabilities.entry(hand).or_insert(Probability(0.0)).0 += probability_per_hand;
    }
    ProbabilitiesToRoll {
        table: probabilities,
    }
}

/// Calculate best reroll
/// # Arguments
/// * `state` - see architecture of structure above
/// * `have` - hand to start with, assumed to be sorted
/// * `rerolls` - rerolls left, e.g. three at beginning of turn
/// * `rules` - rules to be used
/// # Returns
/// Reroll recommendation - see architecture of structure above
#[cached(
    key = "String",
    convert = r#"{ format!("{}{}{},{}", state.compact_fmt(), rules.short_name, hand.compact_fmt(), rerolls) }"#
)]
pub fn choose_reroll(
    state: &State,
    hand: &PartialHand,
    rerolls: Rerolls,
    rules: &rules::Rules,
) -> RerollRecomm {
    // End of turn or chip used
    if rerolls == 0 || rerolls == -2 {
        let stop_now = choose_field(state, hand, rules);
        // Try chip if we have some left and have not used one already
        if state.chips > 0 && rerolls == 0 {
            let mut chip_off = state.clone();
            chip_off.chips -= 1;
            let use_chip = choose_reroll(&chip_off, hand, rerolls - 1, rules);
            if use_chip.expectation > stop_now.expectation {
                return use_chip;
            }
        }
        return RerollRecomm {
            hand: hand.clone(),
            state: stop_now.state,
            expectation: stop_now.expectation,
        };
    }

    struct HandChance {
        hand: PartialHand,
        expectation: Expectation,
    }

    let dice_rules = &rules.dice;
    let mut possible_hands = vec![PartialHand(Vec::new())];
    for &el in &hand.0 {
        possible_hands.extend(
            possible_hands
                .clone()
                .into_iter()
                .map(|mut hand| {
                    hand.0.push(el);
                    hand
                })
                .collect::<Vec<PartialHand>>(),
        )
    }
    let best = possible_hands
        .into_par_iter()
        .map(|partial_hand| HandChance {
            hand: partial_hand.clone(),
            expectation: if partial_hand.is_full_hand(&dice_rules.dice) {
                // recommendation to stop, no need to recalculate
                choose_field(state, hand, rules).expectation
            } else {
                // expectation of this choice is all chances of hands multiplied with their
                // expecation values summed up
                probability_to_roll(partial_hand, dice_rules)
                    .table
                    .iter()
                    .map(|(hand, probability)| {
                        let reroll = choose_reroll(state, hand, rerolls - 1, rules);
                        probability.0 * reroll.expectation
                    })
                    .sum()
            },
        })
        .reduce_with(|a, b| if a.expectation > b.expectation { a } else { b })
        .unwrap();
    RerollRecomm {
        hand: best.hand,
        state: state.clone(),
        expectation: best.expectation,
    }
}

/// Calculate best choice of field at end of turn
/// # Arguments
/// * `state` - see architecture of structure above
/// * `hand` - hand to work with
/// * `rules` - rules to be used
/// # Returns
/// Field recommendation - see architecture of structure above
#[cached(
    key = "String",
    convert = r#"{ format!("{}{}{}", state.compact_fmt(), rules.short_name, have.compact_fmt()) }"#
)]
pub fn choose_field(state: &State, have: &PartialHand, rules: &rules::Rules) -> FieldRecomm {
    let fields_rules = &rules.fields;

    let hand: Hand = have.0.iter().map(|&(_, pip)| pip).collect();
    let mut available_fields: Vec<_> = state
        .used
        .iter()
        .enumerate()
        .flat_map(|(section_idx, section)| {
            let new_state = state.clone();
            section
                .iter()
                .enumerate()
                .filter_map(move |(field_idx, field)| {
                    // Consider only if field is unused
                    (!field).then(|| FieldRecomm {
                        section: section_idx,
                        field: field_idx,
                        expectation: 0.0,
                        state: new_state.clone(),
                    })
                })
        })
        .collect();
    let yahtzee_bonus = state.scored_yahtzee
        && rules.yahtzee_bonus != bonus::NONE
        && (fields_rules[LS][YAHTZEE_INDEX].function)(&hand) > 0;
    if available_fields.len() == 1 {
        // End of game
        let last_field = available_fields.pop().unwrap();
        let section = last_field.section;
        let field = last_field.field;

        let mut final_state = state.clone();
        let (score, bonus) = match yahtzee_bonus {
            true => (rules.yahtzee_bonus.rules)(&state.used, hand[0], section, field),
            _ => ((fields_rules[section][field].function)(&hand), 0),
        };
        final_state.score[section] += score;
        final_state.score[LS] += bonus;
        // Apply upper section bonus
        if final_state.score[US] >= rules.us_bonus.threshold {
            final_state.score[US] += rules.us_bonus.bonus;
        }
        final_state.used[section][field] = true;

        let expectation = final_state.score.iter().sum::<Score>() as Expectation;
        return FieldRecomm {
            section,
            field,
            state: final_state,
            expectation,
        };
    }
    available_fields
        .into_par_iter()
        .map(|option| {
            let section = option.section;
            let field = option.field;

            let (score, bonus) = match yahtzee_bonus {
                true => (rules.yahtzee_bonus.rules)(&state.used, hand[0], section, field),
                _ => ((fields_rules[section][field].function)(&hand), 0),
            };
            let mut new_state = state.clone();
            new_state.score[section] += score;
            new_state.score[LS] += bonus;
            new_state.used[section][field] = true;
            if score > 0 && section == LS && field == YAHTZEE_INDEX {
                // Mark Yahtzee bonus available
                new_state.scored_yahtzee = true
            }

            let hand = PartialHand(Vec::new());
            let expectation = choose_reroll(&new_state, &hand, REROLLS, rules).expectation;
            FieldRecomm {
                section,
                field,
                state: new_state,
                expectation,
            }
        })
        .reduce_with(|a, b| if a.expectation > b.expectation { a } else { b })
        .unwrap()
}

/// Logic for dumping and restoring caches (necessary parts only, no disk; see crate::caching)
pub mod persistent_caches {
    use super::*;

    use cached::Cached;
    use once_cell::sync::Lazy;
    use rayon::join;

    /// Caches to be stored
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    pub struct Caches {
        pub probability_to_roll: HashMap<String, ProbabilitiesToRoll>,
        pub choose_reroll: HashMap<String, RerollRecomm>,
        pub choose_field: HashMap<String, FieldRecomm>,
    }

    pub fn dump_caches() -> Caches {
        macro_rules! dump {
            ($cache:ident) => {
                Lazy::force(&$cache).lock().unwrap().get_store().clone()
            };
        }
        let (probability_to_roll, (choose_reroll, choose_field)) = join(
            || dump!(PROBABILITY_TO_ROLL),
            || join(|| dump!(CHOOSE_REROLL), || dump!(CHOOSE_FIELD)),
        );
        Caches {
            probability_to_roll,
            choose_reroll,
            choose_field,
        }
    }

    pub fn populate_caches(caches: Caches) {
        macro_rules! populate {
            ($cache:ident, $dump:expr) => {
                let mut locked = Lazy::force(&$cache).lock().unwrap();
                for (k, v) in $dump {
                    locked.cache_set(k, v);
                }
            };
        }
        let (probability_to_roll, choose_reroll, choose_field) = (
            caches.probability_to_roll,
            caches.choose_reroll,
            caches.choose_field,
        );
        join(
            || {
                populate!(PROBABILITY_TO_ROLL, probability_to_roll);
            },
            || {
                join(
                    || {
                        populate!(CHOOSE_REROLL, choose_reroll);
                    },
                    || {
                        populate!(CHOOSE_FIELD, choose_field);
                    },
                )
            },
        );
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::hands;

    /// Very simple game rules for testing:
    /// One coin, you have to throw a 2, which awards you one point
    pub fn very_simple_rules() -> rules::Rules {
        rules::Rules {
            short_name: 'y',
            dice: rules::DiceRules {
                short_name: 'y',
                dice: Dice(vec![((1, 2), 1)]),
            },
            chips: 2,
            fields: [
                Vec::new(),
                vec![rules::SectionRule {
                    name: "Throw 2".to_string(),
                    function: |hand| (hand[0] - 1) as Score,
                }],
            ],
            us_bonus: rules::USBonusRules {
                threshold: 2,
                bonus: 0,
            },
            yahtzee_bonus: bonus::NONE,
        }
    }

    /// Very simple state corresponding to [`very_simple_rules()`](very_simple_rules)
    pub fn very_simple_state() -> State {
        State::new_from_rules(&very_simple_rules())
    }

    #[test]
    fn test_compact_fmt_partial_hand() {
        assert_eq!(PartialHand(Vec::new()).compact_fmt(), "");
        assert_eq!(
            PartialHand(vec![((1, 2), 1), ((1, 2), 2)]).compact_fmt(),
            "1,2,1,1,2,2"
        );
    }

    #[test]
    fn test_new_state_from_rules() {
        assert_eq!(
            State::new_from_rules(&very_simple_rules()),
            State {
                score: [0, 0],
                used: [Vec::new(), vec![false]],
                scored_yahtzee: false,
                chips: 2,
            }
        );
    }

    #[test]
    fn test_compact_fmt_state() {
        assert_eq!(
            State {
                score: [0, 0],
                used: [vec![false], vec![false, false]],
                scored_yahtzee: false,
                chips: 0,
            }
            .compact_fmt(),
            "0,0,0,000,0",
        );
    }

    #[test]
    fn test_probability_to_roll() {
        // play with four coins, three left to throw
        // comparing probabilities for equality is okay when comparing eighths
        assert_eq!(
            probability_to_roll(
                PartialHand(vec![((1, 2), 1)]),
                &rules::DiceRules {
                    short_name: 'w',
                    dice: Dice(vec![((1, 2), 4)]),
                }
            ),
            ProbabilitiesToRoll {
                table: [
                    (
                        PartialHand(vec![((1, 2), 1), ((1, 2), 1), ((1, 2), 1), ((1, 2), 1)]),
                        Probability(0.125),
                    ),
                    (
                        PartialHand(vec![((1, 2), 1), ((1, 2), 1), ((1, 2), 1), ((1, 2), 2)]),
                        Probability(0.375),
                    ),
                    (
                        PartialHand(vec![((1, 2), 1), ((1, 2), 1), ((1, 2), 2), ((1, 2), 2)]),
                        Probability(0.375),
                    ),
                    (
                        PartialHand(vec![((1, 2), 1), ((1, 2), 2), ((1, 2), 2), ((1, 2), 2)]),
                        Probability(0.125),
                    ),
                ]
                .iter()
                .cloned()
                .collect()
            }
        );
    }

    #[test]
    #[should_panic]
    fn test_probability_to_roll_panic() {
        // Running with a mismatch between `have` and `rules` should fail
        probability_to_roll(
            PartialHand(vec![((1, 6), 1)]),
            &rules::DiceRules {
                short_name: 'x',
                dice: Dice(Vec::new()),
            },
        );
    }

    #[test]
    fn test_choose_reroll() {
        let rules = very_simple_rules();
        let mut state = very_simple_state();

        let ready_hand = PartialHand(vec![((1, 2), 2)]);
        let unready_hand = PartialHand(vec![((1, 2), 1)]);
        let empty_hand = PartialHand(Vec::new());

        // With a reroll and a 2 thrown, no reroll should happen
        let rec = choose_reroll(&state, &ready_hand, 1, &rules);
        assert_eq!(rec.hand, ready_hand.clone());
        assert_eq!(rec.expectation, 1.0);

        // With no rerolls and no 2 thrown yet, the chip should be used
        // However, only one chip can be used
        let rec = choose_reroll(&state, &unready_hand, 0, &rules);
        assert_eq!(rec.hand, empty_hand);
        assert_eq!(rec.state.chips, 1);
        assert_eq!(rec.expectation, 0.5);

        // With no rerolls and a 2 thrown, the chip should not be used
        let rec = choose_reroll(&state, &ready_hand, 0, &rules);
        assert_eq!(rec.hand, ready_hand.clone());
        assert_eq!(rec.state.chips, 2);
        assert_eq!(rec.expectation, 1.0);

        // With a reroll and no 2 thrown, the reroll should be used
        // Simpler assuming no chips
        state.chips = 0;
        let rec = choose_reroll(&state, &unready_hand, 1, &rules);
        assert_eq!(rec.hand, empty_hand);
        assert_eq!(rec.expectation, 0.5);
    }

    #[test]
    fn test_choose_field() {
        // Dummy section rule to fill rules before Yahtzee
        fn dummy_section_rule() -> rules::SectionRule {
            rules::SectionRule {
                name: String::from("Dummy"),
                function: |_| 0,
            }
        }

        // Simple game rules for testing
        // Two coins, LS has blanks, chance, and Yahtzee
        // US bonus of 1 for 1
        // Yahtzee bonus is 1, always counts as 4
        let simple_rules = rules::Rules {
            short_name: 'z',
            dice: rules::DiceRules {
                short_name: 'z',
                dice: Dice(vec![((1, 2), 2)]),
            },
            chips: 0,
            fields: [
                vec![
                    rules::SectionRule {
                        name: String::from("Count and Add Only Aces"),
                        function: |hand| hands::generic_upper_section(1, hand),
                    },
                    rules::SectionRule {
                        name: String::from("Count and Add Only Twos"),
                        function: |hand| hands::generic_upper_section(2, hand),
                    },
                ],
                vec![
                    // Cloning is more complicated
                    dummy_section_rule(),
                    dummy_section_rule(),
                    dummy_section_rule(),
                    dummy_section_rule(),
                    rules::SectionRule {
                        name: String::from("Chance"),
                        function: hands::total,
                    },
                    // Yahtzee field
                    rules::SectionRule {
                        name: String::from("All Twos"),
                        function: |hand| if hands::total(hand) == 4 { 4 } else { 0 },
                    },
                ],
            ],
            us_bonus: rules::USBonusRules {
                threshold: 1,
                bonus: 1,
            },
            yahtzee_bonus: bonus::Rules {
                short_name: 'z',
                rules: |_, _, _, _| (4, 1),
            },
        };

        let pair_of_twos = PartialHand([((1, 2), 2)].repeat(2));
        let ls_full_except_chance = [[true].repeat(4), vec![false], vec![true]].concat();

        // Pair of Twos hits lower expectation value with All Twos, but it is not available,
        // but it also scores higher than Count Aces, so Chance should be used.
        let mut state = State::new_from_rules(&simple_rules);
        // Some base score out of thin air to ensure it is really added
        state.score[1] = 1;
        state.used = [vec![false, true], ls_full_except_chance.clone()];
        let rec = choose_field(&state, &pair_of_twos, &simple_rules);
        assert_eq!(rec.section, LS);
        assert_eq!(rec.field, 4);
        assert_eq!(rec.state.score[LS], 4 + 1);
        assert_eq!(rec.state.used[LS][4], true);

        // Pair of Twos hits lower expectation value with All Twos,
        // so it should be used over Chance
        state.used = [
            [true].repeat(2),
            [[true].repeat(4), [false].repeat(2)].concat(),
        ];
        let rec = choose_field(&state, &pair_of_twos, &simple_rules);
        assert_eq!(rec.section, LS);
        assert_eq!(rec.field, YAHTZEE_INDEX);
        assert_eq!(rec.state.score[LS], 4 + 1);
        assert_eq!(rec.state.used[LS][YAHTZEE_INDEX], true);
        assert_eq!(rec.state.scored_yahtzee, true);

        // Test awardation of upper section bonus
        state.used = [vec![false, true], [true].repeat(6)];
        let hand = PartialHand(vec![((1, 2), 1), ((1, 2), 2)]);
        let rec = choose_field(&state, &hand, &simple_rules);
        assert_eq!(rec.section, US);
        assert_eq!(rec.field, 0);
        assert_eq!(rec.state.score[US], 2);
        // not asserting rec.state.used -- don't care at this point

        // Test no awardation of upper section bonus
        let rec = choose_field(&state, &pair_of_twos, &simple_rules);
        assert_eq!(rec.section, US);
        assert_eq!(rec.field, 0);
        assert_eq!(rec.state.score[US], 0);

        // Test awardation of Yahtzee bonus
        state.used = [[true].repeat(2), ls_full_except_chance.clone()];
        state.scored_yahtzee = true;
        let rec = choose_field(&state, &pair_of_twos, &simple_rules);
        assert_eq!(rec.section, LS);
        assert_eq!(rec.field, 4);
        assert_eq!(rec.state.score[LS], 4 + 1 + 1);
    }
}
