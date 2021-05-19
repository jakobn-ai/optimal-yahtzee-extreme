use std::collections::HashMap;

use crate::global::*;

/// Rules for Yahtzee bonus
/// # Arguments
/// * Scorecard to check for eligibility
/// * Pip that the Yahtzee was (e.g. a Yahtzee of Fives) to check for eligibility
/// # Returns
/// * Possible scorecards after applying bonus
pub type Rules = fn(&ScoreCard, Pip) -> Vec<ScoreCard>;

/// Score of a Yahtzee bonus
pub const YAHTZEE_BONUS: Score = 100;

/// Indices and scores of fields where Yahtzee might be used as a joker
/// (Full House, Small & Large Straight)
#[inline]
fn joker_fields() -> HashMap<usize, Score> {
    [(2, 25), (3, 30), (4, 40)].iter().cloned().collect()
}

/// Check if player is eligible for bonus
/// (is not eligible when Yahtzee was marked zero or is not yet filled)
#[inline]
fn eligible(score_card: &ScoreCard) -> bool {
    score_card[1][YAHTZEE_INDEX] > 0
}

/// Forced Joker rules, used in regular Yahtzee
pub const FORCED_JOKER: Rules = |score_card, pip| {
    if !eligible(&score_card) {
        return vec![];
    }

    // We are eligible, apply bonus for all possibilities
    let mut score_card_copy = score_card.clone();
    score_card_copy[1][YAHTZEE_INDEX] += YAHTZEE_BONUS;

    let upper_section_pip_index = (pip - 1) as usize;
    if score_card[0][upper_section_pip_index] < 0 {
        // Upper section was unused, must use
        score_card_copy[0][upper_section_pip_index] = YAHTZEE_SIZE as Score * pip as Score;
        return vec![score_card_copy];
    }

    let mut score_cards = Vec::<ScoreCard>::new();
    let joker_fields = joker_fields();
    // Cannot apply Yahtzee bonus to Yahtzee itself, cannot use used fields
    for field in
        (0..LS_LENGTH).filter(|&field| field != YAHTZEE_INDEX && score_card_copy[1][field] < 0)
    {
        let mut bonus_copy = score_card_copy.clone();
        bonus_copy[1][field] = match joker_fields.get(&field) {
            // Bonus
            Some(&score) => score,
            // Count all
            None => YAHTZEE_SIZE as Score * pip as Score,
        };
        score_cards.push(bonus_copy);
    }

    // Must zero one in upper section if lower section was all full
    if score_cards.is_empty() {
        for field in (0..US_LENGTH).filter(|&field| score_card_copy[0][field] < 0) {
            let mut bonus_copy = score_card_copy.clone();
            bonus_copy[0][field] = 0;
            score_cards.push(bonus_copy);
        }
    }

    score_cards
};

/// No Yahtzee bonus, Yahtzee Extreme
pub const NONE: Rules = |_, _| vec![];

// TODO add Free Joker, original rules, Kniffel rules
