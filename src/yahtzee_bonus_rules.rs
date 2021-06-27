// due to additional bonus rules, TODO use
#![allow(dead_code)]

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
    score_card[LS][YAHTZEE_INDEX] > 0
}

/// Forced Joker rules, used in regular Yahtzee
pub const FORCED_JOKER: Rules = |score_card, pip| {
    if !eligible(&score_card) {
        return vec![];
    }

    // We are eligible, apply bonus for all possibilities
    let mut score_card_copy = score_card.clone();
    score_card_copy[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;

    let upper_section_pip_index = (pip - 1) as usize;
    if score_card[US][upper_section_pip_index] < 0 {
        // Upper section was unused, must use
        score_card_copy[US][upper_section_pip_index] = (YAHTZEE_SIZE * pip) as Score;
        return vec![score_card_copy];
    }

    let mut score_cards = Vec::<ScoreCard>::new();
    let joker_fields = joker_fields();
    // Cannot use used fields
    for field in (0..LS_LENGTH).filter(|&field| score_card_copy[LS][field] < 0) {
        let mut bonus_copy = score_card_copy.clone();
        bonus_copy[LS][field] = match joker_fields.get(&field) {
            // Bonus
            Some(&score) => score,
            // Count all
            None => (YAHTZEE_SIZE * pip) as Score,
        };
        score_cards.push(bonus_copy);
    }

    // Must zero one in upper section if lower section was all full
    if score_cards.is_empty() {
        for field in (0..US_LENGTH).filter(|&field| score_card_copy[US][field] < 0) {
            let mut bonus_copy = score_card_copy.clone();
            bonus_copy[US][field] = 0;
            score_cards.push(bonus_copy);
        }
    }

    score_cards
};

/// Free Joker rules, a popular alternative
pub const FREE_JOKER: Rules = |score_card, pip| {
    if !eligible(&score_card) {
        return vec![];
    }

    let mut score_card_copy = score_card.clone();
    score_card_copy[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
    let mut score_cards = Vec::<ScoreCard>::new();

    let mut upper_section_was_full = true;
    let upper_section_pip_index = (pip - 1) as usize;
    if score_card[US][upper_section_pip_index] < 0 {
        // Upper section was unused, can use
        let mut bonus_copy = score_card_copy.clone();
        bonus_copy[US][upper_section_pip_index] = (YAHTZEE_SIZE * pip) as Score;
        score_cards.push(bonus_copy);
        upper_section_was_full = false;
    }

    let joker_fields = joker_fields();
    for field in (0..LS_LENGTH).filter(|&field| score_card_copy[LS][field] < 0) {
        let mut bonus_copy = score_card_copy.clone();
        let joker_score = joker_fields.get(&field);
        if let Some(score) = &joker_score {
            if upper_section_was_full {
                bonus_copy[LS][field] = **score;
                score_cards.push(bonus_copy);
            }
        } else {
            bonus_copy[LS][field] = (YAHTZEE_SIZE * pip) as Score;
            score_cards.push(bonus_copy);
        }
    }

    if score_cards.is_empty() {
        for field in (0..US_LENGTH).filter(|&field| score_card_copy[US][field] < 0) {
            let mut bonus_copy = score_card_copy.clone();
            bonus_copy[US][field] = 0;
            score_cards.push(bonus_copy);
        }
    }

    score_cards
};

/// No Yahtzee bonus, Yahtzee Extreme
pub const NONE: Rules = |_, _| vec![];

/// Kniffel rules, as published in German-speaking countries
pub const KNIFFEL: Rules = |score_card, pip| {
    if !eligible(&score_card) {
        return vec![];
    }

    let mut score_card_copy = score_card.clone();
    // Award bonus of 50 (not 100)
    score_card_copy[LS][YAHTZEE_INDEX] += YAHTZEE_SCORE;

    // Fill Upper Section field if it's still free
    let upper_section_pip_index = (pip - 1) as usize;
    if score_card[US][upper_section_pip_index] < 0 {
        score_card_copy[US][upper_section_pip_index] = (YAHTZEE_SIZE * pip) as Score;
        return vec![score_card_copy];
    }

    // Must zero another field
    let mut score_cards = Vec::<ScoreCard>::new();
    for &(section, length) in [(0, US_LENGTH), (1, LS_LENGTH)].iter() {
        for field in (0..length).filter(|&field| score_card_copy[section][field] < 0) {
            let mut bonus_copy = score_card_copy.clone();
            bonus_copy[section][field] = 0;
            score_cards.push(bonus_copy);
        }
    }

    score_cards
};

#[cfg(test)]
mod tests {
    use super::*;

    fn test_common(rules: Rules) {
        // Yahtzee has not yet been scored, no bonus
        let empty_scorecard = [vec![-1; US_LENGTH], vec![-1; LS_LENGTH]];
        let empty_scorecard_vec = Vec::<ScoreCard>::new();
        assert_eq!(rules(&empty_scorecard, 1), empty_scorecard_vec);

        // Yahtzee was zeroed, no bonus
        let mut zeroed_yahtzee = empty_scorecard.clone();
        zeroed_yahtzee[LS][YAHTZEE_INDEX] = 0;
        assert_eq!(rules(&zeroed_yahtzee, 1), empty_scorecard_vec);
    }

    fn test_common_zeroing_and_two_bonuses(rules: Rules) {
        // Pip in upper section was used and lower section is full, we must zero one in upper
        // section
        let mut full = [vec![-1; US_LENGTH], vec![0; LS_LENGTH]];
        full[US][0] = 0;
        full[LS][YAHTZEE_INDEX] = YAHTZEE_SCORE;
        let mut expected_full = full.clone();
        expected_full[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        let mut expected_fulls = Vec::<ScoreCard>::new();
        for field in 1..US_LENGTH {
            let mut use_field = expected_full.clone();
            use_field[US][field] = 0;
            expected_fulls.push(use_field);
        }
        assert_eq!(rules(&full, 1), expected_fulls);

        // Win bonus twice
        let mut two_bonuses = full.clone();
        two_bonuses[US][0] = 5;
        two_bonuses[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        let mut expected_two_bonuses = two_bonuses.clone();
        expected_two_bonuses[US][1] = 10;
        expected_two_bonuses[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        assert_eq!(rules(&two_bonuses, 2), vec![expected_two_bonuses]);
    }

    #[test]
    fn test_forced_joker() {
        test_common(FORCED_JOKER);

        let mut have_yahtzee = [vec![-1; US_LENGTH], vec![-1; LS_LENGTH]];
        have_yahtzee[LS][YAHTZEE_INDEX] = YAHTZEE_SCORE;

        // Pip in upper section was not used, we must use upper section
        let mut expected_upper_section = have_yahtzee.clone();
        // Bonus Yahtzee of Aces gives 5 in Count and Add Only Aces and the bonus
        expected_upper_section[US][0] = 5;
        expected_upper_section[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        assert_eq!(FORCED_JOKER(&have_yahtzee, 1), vec![expected_upper_section]);

        // Pip in upper section was used, we are allowed to use lower section
        let mut full_upper_section = have_yahtzee.clone();
        full_upper_section[US][0] = 1;
        // Fill Chance in lower section to check it is not overwritten
        full_upper_section[LS][LS_LENGTH - 1] = 10;
        let mut expected_full_upper_section = full_upper_section.clone();
        expected_full_upper_section[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        let mut expected_full_upper_sections = Vec::<ScoreCard>::new();
        for (field, &score) in [5, 5, 25, 30, 40].iter().enumerate() {
            let mut use_field = expected_full_upper_section.clone();
            use_field[LS][field] = score;
            expected_full_upper_sections.push(use_field);
        }
        assert_eq!(
            FORCED_JOKER(&full_upper_section, 1),
            expected_full_upper_sections
        );

        test_common_zeroing_and_two_bonuses(FORCED_JOKER);
    }

    #[test]
    fn test_free_joker() {
        test_common(FREE_JOKER);

        let mut have_yahtzee = [vec![-1; US_LENGTH], vec![-1; LS_LENGTH]];
        have_yahtzee[LS][YAHTZEE_INDEX] = YAHTZEE_SCORE;

        let mut unused_upper_section = have_yahtzee.clone();
        unused_upper_section[LS][LS_LENGTH - 1] = 10;
        let mut expected_unused_upper_section = unused_upper_section.clone();
        expected_unused_upper_section[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        // We can use the upper section
        let mut use_upper_section = expected_unused_upper_section.clone();
        use_upper_section[US][0] = 5;
        let mut expected_unused_upper_sections = vec![use_upper_section];
        // Likewise, we can use the lower section, but not with a bonus
        for (field, &score) in [5; 2].iter().enumerate() {
            let mut use_field = expected_unused_upper_section.clone();
            use_field[LS][field] = score;
            expected_unused_upper_sections.push(use_field);
        }
        assert_eq!(
            FREE_JOKER(&unused_upper_section, 1),
            expected_unused_upper_sections
        );

        let mut full_upper_section = have_yahtzee.clone();
        full_upper_section[US][0] = 5;
        let mut expected_full_upper_section = full_upper_section.clone();
        expected_full_upper_section[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        let mut expected_full_upper_sections = Vec::<ScoreCard>::new();
        // With the field filled in the upper section, can now use bonuses in the lower section
        for (field, &score) in [5, 5, 25, 30, 40, 50, 5]
            .iter()
            .enumerate()
            .filter(|&(field, _)| field != YAHTZEE_INDEX)
        {
            let mut use_field = expected_full_upper_section.clone();
            use_field[LS][field] = score;
            expected_full_upper_sections.push(use_field);
        }
        assert_eq!(
            FREE_JOKER(&full_upper_section, 1),
            expected_full_upper_sections
        );

        test_common_zeroing_and_two_bonuses(FREE_JOKER);
    }

    #[test]
    fn test_none() {
        let mut test_yahtzee_bonus_ls = vec![-1; 16];
        // add a Yahtzee just to make sure
        test_yahtzee_bonus_ls[10] = 50;
        assert_eq!(
            NONE(&[vec![-1; US_LENGTH], test_yahtzee_bonus_ls], 1),
            Vec::<ScoreCard>::new()
        );
    }

    #[test]
    fn test_kniffel() {
        test_common(KNIFFEL);

        let mut have_yahtzee = [vec![-1; US_LENGTH], vec![-1; LS_LENGTH]];
        have_yahtzee[LS][YAHTZEE_INDEX] = YAHTZEE_SCORE;

        // Pip in upper section still available, must be used
        let mut expected_available = have_yahtzee.clone();
        expected_available[US][0] = 5;
        expected_available[LS][YAHTZEE_INDEX] += YAHTZEE_SCORE;
        assert_eq!(KNIFFEL(&have_yahtzee, 1), vec![expected_available]);

        // Pip in upper section was used, must zero
        let mut was_used = have_yahtzee.clone();
        was_used[US][0] = 1;
        let mut was_used_bonus = was_used.clone();
        was_used_bonus[LS][YAHTZEE_INDEX] += YAHTZEE_SCORE;
        let mut expected_used: Vec<ScoreCard> = (1..US_LENGTH)
            .map(|field| {
                let mut card = was_used_bonus.clone();
                card[US][field] = 0;
                card
            })
            .collect();
        expected_used.append(
            &mut (0..LS_LENGTH)
                .filter(|&field| field != YAHTZEE_INDEX)
                .map(|field| {
                    let mut card = was_used_bonus.clone();
                    card[LS][field] = 0;
                    card
                })
                .collect(),
        );
        assert_eq!(KNIFFEL(&was_used, 1), expected_used);
    }
}

// TODO add original rules