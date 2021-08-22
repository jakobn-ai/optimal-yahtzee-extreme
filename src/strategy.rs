// to quiet warnings, TODO use
#![allow(dead_code)]

use std::collections::hash_map::RandomState;
use std::collections::HashMap;

use crate::global::*;
use crate::rules;
use crate::yahtzee_bonus_rules;

/// Partial hand, specifying dice and pips
type PartialHand = Vec<(Die, Pip)>;
#[cfg(target_pointer_width = "64")]
type ArchFloat = f64;
#[cfg(target_pointer_width = "32")]
type ArchFloat = f32;
/// Statistical probability
type Probability = ArchFloat;
/// Expectation value
type Expectation = ArchFloat;

/// State with everything relevant to strategy
struct State<'a> {
    // We could _almost_ make the rules static (or lazy_static) and would not have carry around all
    // these references, but they have to be mutable a _few_ times, so I am not sure whether this
    // can be done without `unsafe`, which I would like to avoid. If you know of a better way,
    // please tell me.
    rules: &'a rules::Rules,
    score: [Score; 2],
    used: ScoreCard,
    scored_yahtzee: bool,
    chips: Chips,
}

impl Clone for State<'_> {
    fn clone(&self) -> Self {
        State {
            rules: self.rules,
            score: self.score,
            used: self.used.clone(),
            scored_yahtzee: self.scored_yahtzee,
            chips: self.chips,
        }
    }
}

/// Recommendation for what to keep for rerolling
struct RerollRecomm<'a> {
    hand: PartialHand,
    state: State<'a>,
    expectation: Expectation,
}

/// Recommendation for which field to use for score
struct FieldRecomm<'a> {
    section: Section,
    field: Field,
    state: State<'a>,
    expectation: Expectation,
}

impl Clone for FieldRecomm<'_> {
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
fn probability_to_roll(
    have: PartialHand,
    rules: &rules::DiceRules,
) -> HashMap<PartialHand, Probability> {
    // Calculate dice left to use
    let mut leftover = rules.clone();
    for (die, _) in &have {
        *leftover.get_mut(die).unwrap() -= 1;
    }

    // Calculate all possible hands
    let mut hands: Vec<PartialHand> = vec![have];
    for (&(min, max), &frequency) in &leftover {
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
        total * (max - min + 1) * frequency
    });
    let probability_per_hand = 1.0 / total as Probability;

    // Sort hands and add up probabilities
    let mut probabilities = HashMap::with_hasher(RandomState::new());
    for mut hand in hands {
        hand.sort_unstable_by_key(|(_, pip)| *pip);
        *probabilities.entry(hand).or_insert(0.0) += probability_per_hand;
    }
    probabilities
}

/// Calculate best reroll
/// # Arguments
/// * `state` - see architecture of structure above
/// * `have` - hand to start with, assumed to be sorted
/// * `rerolls` - rerolls left, e.g. three at beginning of turn
/// # Returns
/// Reroll recommendation - see architecture of structure above
fn choose_reroll(state: State, hand: PartialHand, rerolls: i8) -> RerollRecomm {
    // End of turn or chip used
    if rerolls == 0 || rerolls == -2 {
        let stop_now = choose_field(state.clone(), hand.clone());
        // Try chip if we have some left and have not used one already
        if state.chips > 0 && rerolls == 0 {
            let mut chip_off = state.clone();
            chip_off.chips -= 1;
            let use_chip = choose_reroll(chip_off, hand.clone(), rerolls - 1);
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

    let dice_rules = &state.rules.dice;
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
        .iter()
        .map(|partial_hand| HandChance {
            hand: partial_hand.to_vec(),
            expectation: if partial_hand.len() == dice_rules.values().sum::<Frequency>() as usize {
                // recommendation to stop, no need to recalculate
                choose_field(state.clone(), hand.clone()).expectation
            } else {
                // expectation of this choice is all chances of hands multiplied with their
                // expecation values summed up
                probability_to_roll(partial_hand.to_vec(), dice_rules)
                    .iter()
                    .map(|(hand, probability)| {
                        let reroll = choose_reroll(state.clone(), hand.to_vec(), rerolls - 1);
                        probability * reroll.expectation
                    })
                    .sum()
            },
        })
        .reduce(|a, b| if a.expectation > b.expectation { a } else { b })
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
/// # Returns
/// Field recommendation - see architecture of structure above
fn choose_field(state: State, have: PartialHand) -> FieldRecomm {
    let rules = state.rules;
    let fields_rules = &rules.fields;

    let hand: Hand = have.iter().map(|(_, pip)| *pip).collect();
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
        && rules.yahtzee_bonus as usize != yahtzee_bonus_rules::NONE as usize
        && (fields_rules[LS][YAHTZEE_INDEX].function)(&hand) > 0;
    if available_fields.len() == 1 {
        // End of game
        let last_field = available_fields.pop().unwrap();
        let section = last_field.section;
        let field = last_field.field;

        let mut final_state = state.clone();
        let (score, bonus) = match yahtzee_bonus {
            true => (rules.yahtzee_bonus)(&state.used, hand[0], section, field),
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
        .iter()
        .map(|option| {
            let section = option.section;
            let field = option.field;

            let (score, bonus) = match yahtzee_bonus {
                true => (rules.yahtzee_bonus)(&state.used, hand[0], section, field),
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

            let expectation = choose_reroll(new_state.clone(), vec![], THROWS).expectation;
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
    fn test_probability_to_roll() {
        // play with three coins, two left to throw
        // comparing probabilities for equality is okay when comparing 1/4 or 1/2
        assert_eq!(
            probability_to_roll(vec![((1, 2), 1)], &[((1, 2), 3)].iter().cloned().collect()),
            [
                (vec![((1, 2), 1), ((1, 2), 1), ((1, 2), 1)], 0.25),
                (vec![((1, 2), 1), ((1, 2), 1), ((1, 2), 2)], 0.5),
                (vec![((1, 2), 1), ((1, 2), 2), ((1, 2), 2)], 0.25)
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
        probability_to_roll(vec![((1, 6), 1)], &HashMap::new());
    }

    #[test]
    fn test_choose_reroll() {
        // Simple game rules for testing
        // One coin, you have to throw a 2, which awards you one point
        let simple_rules = rules::Rules {
            dice: [((1, 2), 1)].iter().cloned().collect(),
            chips: 2,
            fields: [
                vec![],
                vec![rules::SectionRule {
                    name: "Throw 1".to_string(),
                    function: Box::new(|hand| (hand[0] - 1) as Score),
                }],
            ],
            us_bonus: rules::USBonusRules {
                threshold: 2,
                bonus: 0,
            },
            yahtzee_bonus: yahtzee_bonus_rules::NONE,
        };

        let ready_hand = vec![((1, 2), 2)];
        let unready_hand = vec![((1, 2), 1)];

        // With no rerolls and no 2 thrown yet, the chip should be used
        // However, only one chip can be used
        let mut state = State {
            rules: &simple_rules,
            score: [0, 0],
            used: [vec![], vec![false]],
            scored_yahtzee: false,
            chips: 2,
        };
        let rec = choose_reroll(state.clone(), unready_hand.clone(), 0);
        assert_eq!(rec.hand, vec![]);
        assert_eq!(rec.state.chips, 1);
        assert_eq!(rec.expectation, 0.5);

        // With no rerolls and a 2 thrown, the chip should not be used
        let rec = choose_reroll(state.clone(), ready_hand.clone(), 0);
        assert_eq!(rec.hand, ready_hand.clone());
        assert_eq!(rec.state.chips, 2);
        assert_eq!(rec.expectation, 1.0);

        // With a reroll and no 2 thrown, the reroll should be used
        // Simpler assuming no chips
        state.chips = 0;
        let rec = choose_reroll(state.clone(), unready_hand.clone(), 1);
        assert_eq!(rec.hand, vec![]);
        assert_eq!(rec.expectation, 0.5);

        // With a reroll and a 2 thrown, no reroll should happen
        let rec = choose_reroll(state.clone(), ready_hand.clone(), 1);
        assert_eq!(rec.hand, ready_hand.clone());
        assert_eq!(rec.expectation, 1.0);
    }

    #[test]
    fn test_choose_field() {
        // Dummy section rule to fill rules before Yahtzee
        fn dummy_section_rule() -> rules::SectionRule {
            rules::SectionRule {
                name: String::from("Dummy"),
                function: Box::new(|_| 0),
            }
        }

        // Simple game rules for testing
        // Two coins, LS has blanks, chance, and Yahtzee
        // US bonus of 1 for 1
        // Yahtzee bonus is 1, always counts as 4
        let simple_rules = rules::Rules {
            dice: [((1, 2), 2)].iter().cloned().collect(),
            chips: 0,
            fields: [
                ["Aces", "Twos"]
                    .iter()
                    .zip(1..3)
                    .map(|(name, field)| rules::SectionRule {
                        name: format!("Count and Add Only {}", name),
                        function: Box::new(move |hand| hands::generic_upper_section(field, hand)),
                    })
                    .collect(),
                vec![
                    // Cannot clone boxed functions
                    dummy_section_rule(),
                    dummy_section_rule(),
                    dummy_section_rule(),
                    dummy_section_rule(),
                    rules::SectionRule {
                        name: String::from("Chance"),
                        function: Box::new(hands::total),
                    },
                    // Yahtzee field
                    rules::SectionRule {
                        name: String::from("All Twos"),
                        function: Box::new(|hand| if hands::total(hand) == 4 { 4 } else { 0 }),
                    },
                ],
            ],
            us_bonus: rules::USBonusRules {
                threshold: 1,
                bonus: 1,
            },
            yahtzee_bonus: |_, _, _, _| (4, 1),
        };

        let pair_of_twos = [((1, 2), 2)].repeat(2);
        let ls_full_except_chance = [[true].repeat(4), vec![false], vec![true]].concat();

        // Pair of Twos hits lower expectation value with All Twos, but it is not available,
        // but it also scores higher than Count Aces, so Chance should be used.
        let mut state = State {
            rules: &simple_rules,
            // Some base score out of thin air to ensure it is really added
            score: [0, 1],
            used: [vec![false, true], ls_full_except_chance.clone()],
            scored_yahtzee: false,
            chips: 0,
        };
        let rec = choose_field(state.clone(), pair_of_twos.clone());
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
        let rec = choose_field(state.clone(), pair_of_twos.clone());
        assert_eq!(rec.section, LS);
        assert_eq!(rec.field, YAHTZEE_INDEX);
        assert_eq!(rec.state.score[LS], 4 + 1);
        assert_eq!(rec.state.used[LS][YAHTZEE_INDEX], true);
        assert_eq!(rec.state.scored_yahtzee, true);

        // Test awardation of upper section bonus
        state.used = [vec![false, true], [true].repeat(6)];
        let rec = choose_field(state.clone(), vec![((1, 2), 1), ((1, 2), 2)]);
        assert_eq!(rec.section, US);
        assert_eq!(rec.field, 0);
        assert_eq!(rec.state.score[US], 2);
        // not asserting rec.state.used -- don't care at this point

        // Test no awardation of upper section bonus
        let rec = choose_field(state.clone(), pair_of_twos.clone());
        assert_eq!(rec.section, US);
        assert_eq!(rec.field, 0);
        assert_eq!(rec.state.score[US], 0);

        // Test awardation of Yahtzee bonus
        state.used = [[true].repeat(2), ls_full_except_chance.clone()];
        state.scored_yahtzee = true;
        let rec = choose_field(state.clone(), pair_of_twos.clone());
        assert_eq!(rec.section, LS);
        assert_eq!(rec.field, 4);
        assert_eq!(rec.state.score[LS], 4 + 1 + 1);
    }
}
