// to quiet warnings, TODO use
#![allow(dead_code)]

use std::collections::HashMap;

use crate::global::*;
use crate::rules::*;

/// Partial hand, specifying dice and pips
type PartialHand = Vec<(Die, Pip)>;
/// Statistical probability
#[cfg(target_pointer_width = "64")]
type Probability = f64;
#[cfg(target_pointer_width = "32")]
type Probability = f32;

/// Probability to roll hands given hand
/// # Arguments
/// * `have` - partial hand to start with
/// * `rules` - dice rules
/// # Returns
/// Hash map of all reachable hands and probabilities, hands sorted
fn probability_to_roll(have: PartialHand, rules: DiceRules) -> HashMap<Vec<Pip>, Probability> {
    // Calculate dice left to use
    let mut leftover = rules;
    for (die, _) in &have {
        *leftover.get_mut(die).unwrap() -= 1;
    }

    // Calculate all possible hands
    let mut hands: Vec<Vec<Pip>> = vec![have.iter().map(|&t| t.1).collect()];
    for (&(min, max), &frequency) in &leftover {
        for _ in 0..frequency {
            hands = hands
                .iter()
                .flat_map(|hand| {
                    (min..(max + 1)).map(move |pip| {
                        // Append possible pip to hand
                        let mut new_hand = hand.clone();
                        new_hand.push(pip);
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
    let mut probabilities = HashMap::new();
    for mut hand in hands {
        hand.sort_unstable();
        *probabilities.entry(hand).or_insert(0.0) += probability_per_hand;
    }
    probabilities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_macros)]
    macro_rules! assert_approx {
        ($a:ident, $b:ident) => {
            assert!(abs(a - b) < Probability::EPSILON);
        };
    }

    #[test]
    fn test_probability_to_roll() {
        // play with three coins, two left to throw
        // comparing probabilities for equality is okay when comparing 1/4 or 1/2
        assert_eq!(
            probability_to_roll(vec![((1, 2), 1)], [((1, 2), 3)].iter().cloned().collect()),
            [
                (vec![1, 1, 1], 0.25),
                (vec![1, 1, 2], 0.5),
                (vec![1, 2, 2], 0.25)
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
        probability_to_roll(vec![((1, 6), 1)], HashMap::new());
    }
}
