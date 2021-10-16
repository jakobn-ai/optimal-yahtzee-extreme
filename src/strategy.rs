// to quiet warnings, TODO use
#![allow(dead_code)]

use std::collections::hash_map::RandomState;
use std::collections::HashMap;

use crate::global::*;
use crate::rules;
use crate::yahtzee_bonus_rules as bonus;

use cached::proc_macro::cached;
use rayon::prelude::*;

/// Partial hand, specifying dice and pips
type PartialHand = Vec<(Die, Pip)>;
/// Partial hand but it makes Clippy happy
type PartialHandSlice = [(Die, Pip)];
#[cfg(target_pointer_width = "64")]
type ArchFloat = f64;
#[cfg(target_pointer_width = "32")]
type ArchFloat = f32;
/// Statistical probability
type Probability = ArchFloat;
/// Expectation value
type Expectation = ArchFloat;

/// Compact formatting for caching
fn compact_fmt(hand: &PartialHandSlice) -> String {
    hand.iter()
        .map(|((min, max), pip)| format!("{},{},{}", min, max, pip))
        .reduce(|a, b| format!("{},{}", a, b))
        .unwrap_or_else(|| String::from(""))
}

/// State with everything relevant to strategy
struct State {
    score: [Score; 2],
    used: ScoreCard,
    scored_yahtzee: bool,
    chips: Chips,
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
    fn compact_fmt(&self) -> String {
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

/// Recommendation for what to keep for rerolling
#[derive(Clone)]
struct RerollRecomm {
    hand: PartialHand,
    state: State,
    expectation: Expectation,
}

/// Recommendation for which field to use for score
struct FieldRecomm {
    section: Section,
    field: Field,
    state: State,
    expectation: Expectation,
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
/// Hash map of all reachable hands and probabilities, hands sorted
// TODO cache on disk
#[cached(
    key = "String",
    convert = r#"{ format!("{}{}", compact_fmt(&have), rules.short_name ) }"#
)]
fn probability_to_roll(
    have: PartialHand,
    rules: &rules::DiceRules,
) -> HashMap<PartialHand, Probability> {
    // Calculate dice left to use
    let mut leftover = rules.dice.to_owned();
    'next_have: for &(have_die, _) in &have {
        for (left_die, freq) in &mut leftover {
            if have_die == *left_die {
                *freq -= 1;
                continue 'next_have;
            }
        }
        panic!("Mismatch between hand and rules");
    }

    // Calculate all possible hands
    let mut hands: Vec<PartialHand> = vec![have];
    for &((min, max), frequency) in &leftover {
        for _ in 0..frequency {
            hands = hands
                .iter()
                .flat_map(|hand| {
                    (min..(max + 1)).map(move |pip| {
                        // Append possible pip to hand
                        let mut new_hand = hand.clone();
                        new_hand.push(((min, max), pip));
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
    let probability_per_hand = 1.0 / total as Probability;

    // Sort hands and add up probabilities
    let mut probabilities = HashMap::with_hasher(RandomState::new());
    for mut hand in hands {
        hand.sort_unstable_by_key(|&(_, pip)| pip);
        *probabilities.entry(hand).or_insert(0.0) += probability_per_hand;
    }
    probabilities
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
    convert = r#"{ format!("{:?}{}{}{}", state.compact_fmt(), rules.short_name, compact_fmt(&hand), rerolls) }"#
)]
fn choose_reroll(
    state: State,
    hand: PartialHand,
    rerolls: i8,
    rules: &rules::Rules,
) -> RerollRecomm {
    // End of turn or chip used
    if rerolls == 0 || rerolls == -2 {
        let stop_now = choose_field(state.clone(), hand.clone(), rules);
        // Try chip if we have some left and have not used one already
        if state.chips > 0 && rerolls == 0 {
            let mut chip_off = state;
            chip_off.chips -= 1;
            let use_chip = choose_reroll(chip_off, hand.clone(), rerolls - 1, rules);
            if use_chip.expectation > stop_now.expectation {
                return use_chip;
            }
        }
        return RerollRecomm {
            hand,
            state: stop_now.state,
            expectation: stop_now.expectation,
        };
    }

    struct HandChance {
        hand: PartialHand,
        expectation: Expectation,
    }

    let dice_rules = &rules.dice;
    let mut possible_hands = vec![vec![]];
    for &el in &hand {
        possible_hands.extend(
            possible_hands
                .clone()
                .into_iter()
                .map(|mut hand| {
                    hand.push(el);
                    hand
                })
                .collect::<Vec<PartialHand>>(),
        )
    }
    let best = possible_hands
        .into_par_iter()
        .map(|partial_hand| HandChance {
            hand: partial_hand.to_vec(),
            expectation: if partial_hand.len()
                == dice_rules
                    .dice
                    .iter()
                    .map(|(_, freq)| freq)
                    .sum::<Frequency>() as usize
            {
                // recommendation to stop, no need to recalculate
                choose_field(state.clone(), hand.clone(), rules).expectation
            } else {
                // expectation of this choice is all chances of hands multiplied with their
                // expecation values summed up
                probability_to_roll(partial_hand.to_vec(), dice_rules)
                    .iter()
                    .map(|(hand, probability)| {
                        let reroll =
                            choose_reroll(state.clone(), hand.to_vec(), rerolls - 1, rules);
                        probability * reroll.expectation
                    })
                    .sum()
            },
        })
        .reduce_with(|a, b| if a.expectation > b.expectation { a } else { b })
        .unwrap();
    RerollRecomm {
        hand: best.hand,
        state,
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
    convert = r#"{ format!("{}{}{}", state.compact_fmt(), rules.short_name, compact_fmt(&have)) }"#
)]
fn choose_field(state: State, have: PartialHand, rules: &rules::Rules) -> FieldRecomm {
    let fields_rules = &rules.fields;

    let hand: Hand = have.iter().map(|&(_, pip)| pip).collect();
    let mut available_fields: Vec<FieldRecomm> = state
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
        && rules.yahtzee_bonus.short_name != bonus::NONE.short_name
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

        let expectation = final_state.score.iter().sum::<Score>() as Expectation;
        return FieldRecomm {
            section,
            field,
            state: final_state,
            expectation,
        };
    }
    available_fields = available_fields
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

            let expectation =
                choose_reroll(new_state.clone(), vec![], THROWS - 1, rules).expectation;
            FieldRecomm {
                section,
                field,
                state: new_state,
                expectation,
            }
        })
        .collect();
    // Return choice with best expectation value
    available_fields
        .iter()
        .reduce(|a, b| if a.expectation > b.expectation { a } else { b })
        .unwrap()
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hands;

    #[test]
    fn test_compact_fmt_partial_hand() {
        assert_eq!(compact_fmt(&vec![]), "");
        assert_eq!(compact_fmt(&vec![((1, 2), 1), ((1, 2), 2)]), "1,2,1,1,2,2");
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
                vec![((1, 2), 1)],
                &rules::DiceRules {
                    short_name: 'w',
                    dice: vec![((1, 2), 4)]
                }
            ),
            [
                (
                    vec![((1, 2), 1), ((1, 2), 1), ((1, 2), 1), ((1, 2), 1)],
                    0.125
                ),
                (
                    vec![((1, 2), 1), ((1, 2), 1), ((1, 2), 1), ((1, 2), 2)],
                    0.375
                ),
                (
                    vec![((1, 2), 1), ((1, 2), 1), ((1, 2), 2), ((1, 2), 2)],
                    0.375
                ),
                (
                    vec![((1, 2), 1), ((1, 2), 2), ((1, 2), 2), ((1, 2), 2)],
                    0.125
                ),
            ]
            .iter()
            .cloned()
            .collect()
        );
    }

    #[test]
    #[should_panic]
    fn test_probability_to_roll_panic() {
        // Running with a mismatch between `have` and `rules` should fail
        probability_to_roll(
            vec![((1, 6), 1)],
            &rules::DiceRules {
                short_name: 'x',
                dice: vec![],
            },
        );
    }

    #[test]
    fn test_choose_reroll() {
        // Simple game rules for testing
        // One coin, you have to throw a 2, which awards you one point
        let simple_rules = rules::Rules {
            short_name: 'y',
            dice: rules::DiceRules {
                short_name: 'y',
                dice: vec![((1, 2), 1)],
            },
            chips: 2,
            fields: [
                vec![],
                vec![rules::SectionRule {
                    name: "Throw 1".to_string(),
                    function: |hand| (hand[0] - 1) as Score,
                }],
            ],
            us_bonus: rules::USBonusRules {
                threshold: 2,
                bonus: 0,
            },
            yahtzee_bonus: bonus::NONE,
        };

        let ready_hand = vec![((1, 2), 2)];
        let unready_hand = vec![((1, 2), 1)];

        // With no rerolls and no 2 thrown yet, the chip should be used
        // However, only one chip can be used
        let mut state = State {
            score: [0, 0],
            used: [vec![], vec![false]],
            scored_yahtzee: false,
            chips: 2,
        };
        let rec = choose_reroll(state.clone(), unready_hand.clone(), 0, &simple_rules);
        assert_eq!(rec.hand, vec![]);
        assert_eq!(rec.state.chips, 1);
        assert_eq!(rec.expectation, 0.5);

        // With no rerolls and a 2 thrown, the chip should not be used
        let rec = choose_reroll(state.clone(), ready_hand.clone(), 0, &simple_rules);
        assert_eq!(rec.hand, ready_hand.clone());
        assert_eq!(rec.state.chips, 2);
        assert_eq!(rec.expectation, 1.0);

        // With a reroll and no 2 thrown, the reroll should be used
        // Simpler assuming no chips
        state.chips = 0;
        let rec = choose_reroll(state.clone(), unready_hand.clone(), 1, &simple_rules);
        assert_eq!(rec.hand, vec![]);
        assert_eq!(rec.expectation, 0.5);

        // With a reroll and a 2 thrown, no reroll should happen
        let rec = choose_reroll(state.clone(), ready_hand.clone(), 1, &simple_rules);
        assert_eq!(rec.hand, ready_hand.clone());
        assert_eq!(rec.expectation, 1.0);
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
                dice: vec![((1, 2), 2)],
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

        let pair_of_twos = [((1, 2), 2)].repeat(2);
        let ls_full_except_chance = [[true].repeat(4), vec![false], vec![true]].concat();

        // Pair of Twos hits lower expectation value with All Twos, but it is not available,
        // but it also scores higher than Count Aces, so Chance should be used.
        let mut state = State {
            // Some base score out of thin air to ensure it is really added
            score: [0, 1],
            used: [vec![false, true], ls_full_except_chance.clone()],
            scored_yahtzee: false,
            chips: 0,
        };
        let rec = choose_field(state.clone(), pair_of_twos.clone(), &simple_rules);
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
        let rec = choose_field(state.clone(), pair_of_twos.clone(), &simple_rules);
        assert_eq!(rec.section, LS);
        assert_eq!(rec.field, YAHTZEE_INDEX);
        assert_eq!(rec.state.score[LS], 4 + 1);
        assert_eq!(rec.state.used[LS][YAHTZEE_INDEX], true);
        assert_eq!(rec.state.scored_yahtzee, true);

        // Test awardation of upper section bonus
        state.used = [vec![false, true], [true].repeat(6)];
        let rec = choose_field(state.clone(), vec![((1, 2), 1), ((1, 2), 2)], &simple_rules);
        assert_eq!(rec.section, US);
        assert_eq!(rec.field, 0);
        assert_eq!(rec.state.score[US], 2);
        // not asserting rec.state.used -- don't care at this point

        // Test no awardation of upper section bonus
        let rec = choose_field(state.clone(), pair_of_twos.clone(), &simple_rules);
        assert_eq!(rec.section, US);
        assert_eq!(rec.field, 0);
        assert_eq!(rec.state.score[US], 0);

        // Test awardation of Yahtzee bonus
        state.used = [[true].repeat(2), ls_full_except_chance.clone()];
        state.scored_yahtzee = true;
        let rec = choose_field(state.clone(), pair_of_twos.clone(), &simple_rules);
        assert_eq!(rec.section, LS);
        assert_eq!(rec.field, 4);
        assert_eq!(rec.state.score[LS], 4 + 1 + 1);
    }
}
