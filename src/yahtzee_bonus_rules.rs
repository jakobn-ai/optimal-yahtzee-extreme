// due to additional bonus rules, TODO use
#![allow(dead_code)]

use std::collections::HashMap;

use crate::global::*;

/// Rules for Yahtzee bonus
/// # Arguments
/// * Scorecard to check for eligibility (overwritten)
/// * Pip that the Yahtzee was (e.g. a Yahtzee of Fives)
/// * Section where bonus is applied
/// * Field where bonus is applied within section (assumed to be free)
pub type Rules = fn(&mut ScoreCard, Pip, Section, Field);

/// Score of a Yahtzee bonus
pub const YAHTZEE_BONUS: Score = 100;

/// Indices and scores of fields where Yahtzee might be used as a joker
/// (Full House, Small & Large Straight)
#[inline]
fn joker_fields() -> HashMap<usize, Score> {
    [(2, 25), (3, 30), (4, 40)].iter().cloned().collect()
}

pub const FORCED_JOKER: Rules = |score_card, pip, section, field| {
    score_card[section][field] = match section {
        US => match pip {
            // `field` is zero-indexed
            pip if pip as usize == field + 1 => (YAHTZEE_SIZE * pip) as Score,
            _ => 0,
        },
        _ => match score_card[US][pip as usize - 1] {
            // Upper section unused, not allowed to use, zeroing
            -1 => 0,
            _ => match joker_fields().get(&field) {
                // Bonus
                Some(&score) => score,
                // Count all
                None => (YAHTZEE_SIZE * pip) as Score,
            },
        },
    };
    // Only award bonus when valid
    if score_card[section][field] != 0 {
        score_card[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
    }
};

pub const NONE: Rules = |_, _, _, _| {
    panic!("Rules NONE should not be applied");
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forced_joker() {
        let mut have_yahtzee = [vec![-1; US_LENGTH], vec![-1; LS_LENGTH]];
        have_yahtzee[LS][YAHTZEE_INDEX] = YAHTZEE_SCORE;

        // Upper section should award points when available
        let mut upper_section_available = have_yahtzee.clone();
        let mut expected = upper_section_available.clone();
        expected[US][0] = 5;
        expected[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        FORCED_JOKER(&mut upper_section_available, 1, 0, 0);
        assert_eq!(upper_section_available, expected);

        // Attempt bonus in upper section for wrong field, should not award points
        let mut wrong_field = have_yahtzee.clone();
        expected = wrong_field.clone();
        expected[US][1] = 0;
        FORCED_JOKER(&mut wrong_field, 1, 0, 1);
        assert_eq!(wrong_field, expected);

        // Attempt bonus in lower section when upper section is still available,
        // should not award points
        let mut upper_section_unused = have_yahtzee.clone();
        expected = upper_section_unused.clone();
        expected[LS][0] = 0;
        FORCED_JOKER(&mut upper_section_unused, 1, 1, 0);
        assert_eq!(upper_section_unused, expected);

        // Lower section should award points when upper section is full
        let mut upper_section_used = have_yahtzee.clone();
        upper_section_used[US][0] = 0;
        expected = upper_section_used.clone();
        expected[LS][0] = 5;
        expected[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        FORCED_JOKER(&mut upper_section_used, 1, 1, 0);
        assert_eq!(upper_section_used, expected);

        // should also work with bonus
        expected[LS][2] = 25;
        expected[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        FORCED_JOKER(&mut upper_section_used, 1, 1, 2);
        assert_eq!(upper_section_used, expected);
    }
}

// TODO free choice, original, Kniffel
