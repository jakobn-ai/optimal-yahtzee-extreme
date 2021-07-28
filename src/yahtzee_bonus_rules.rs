// due to additional bonus rules, TODO use
#![allow(dead_code)]

use std::collections::HashMap;

use crate::global::*;

/// Rules for Yahtzee bonus
/// # Arguments
/// * Score card
/// * Pip that the Yahtzee was (e.g. a Yahtzee of Fives)
/// * Section where score is applied
/// * Field where score is applied within section (assumed to be free)
/// # Returns
/// * Score awarded in specified field
/// * Bonus awarded
pub type Rules = fn(&ScoreCard, Pip, Section, Field) -> (Score, Score);

/// Score of a Yahtzee bonus
pub const YAHTZEE_BONUS: Score = 100;

/// Indices and scores of fields where Yahtzee might be used as a joker
/// (Full House, Small & Large Straight)
#[inline]
fn joker_fields() -> HashMap<usize, Score> {
    [
        (2, FULL_HOUSE_SCORE),
        (3, SMALL_STRAIGHT_SCORE),
        (4, LARGE_STRAIGHT_SCORE),
    ]
    .iter()
    .cloned()
    .collect()
}

/// Forced Joker rules, used in regular Yahtzee
pub const FORCED_JOKER: Rules = |score_card, pip, section, field| {
    match section {
        US => match pip {
            // `field` is zero-indexed
            pip if pip as usize == field + 1 => ((YAHTZEE_SIZE * pip) as Score, YAHTZEE_BONUS),
            _ => (0, YAHTZEE_BONUS),
        },
        _ => match score_card[US][pip as usize - 1] {
            // Upper section unused, not allowed to use, zeroing
            false => (0, 0),
            _ => match joker_fields().get(&field) {
                // Joker
                Some(&score) => (score, YAHTZEE_BONUS),
                // Count all
                None => ((YAHTZEE_SIZE * pip) as Score, YAHTZEE_BONUS),
            },
        },
    }
};

/// Free Joker rules, a popular alternative
pub const FREE_JOKER: Rules = |score_card, pip, section, field| {
    match section {
        US => match pip {
            pip if pip as usize == field + 1 => ((YAHTZEE_SIZE * pip) as Score, YAHTZEE_BONUS),
            _ => (0, YAHTZEE_BONUS),
        },
        _ => match joker_fields().get(&field) {
            Some(&score) => match score_card[US][pip as usize - 1] {
                // Upper section unused, not allowed to use joker
                false => (0, 0),
                _ => (score, YAHTZEE_BONUS),
            },
            None => ((YAHTZEE_SIZE * pip) as Score, YAHTZEE_BONUS),
        },
    }
};

/// Original 1956 rules
pub const ORIGINAL: Rules = |_, pip, section, field| {
    match section {
        // Upper section cannot be used
        US => match pip {
            pip if pip as usize == field + 1 => ((YAHTZEE_SIZE * pip) as Score, 0),
            _ => (0, 0),
        },
        _ => match joker_fields().get(&field) {
            Some(&score) => (score, YAHTZEE_BONUS),
            None => ((YAHTZEE_SIZE * pip) as Score, YAHTZEE_BONUS),
        },
    }
};

/// Kniffel rules, as published in German-speaking countries
pub const KNIFFEL: Rules = |score_card, pip, section, field| {
    match section {
        US => match pip {
            pip if pip as usize == field + 1 => ((YAHTZEE_SIZE * pip) as Score, YAHTZEE_SCORE),
            _ => (0, YAHTZEE_SCORE),
        },
        _ => match score_card[US][pip as usize - 1] {
            // TODO should we permit forgoing bonus here?
            false => (0, 0),
            _ => match joker_fields().get(&field) {
                // No joker
                Some(_) => (0, 0),
                None => ((YAHTZEE_SIZE * pip) as Score, YAHTZEE_SCORE),
            },
        },
    }
};

/// No Yahtzee bonus, Yahtzee extreme
pub const NONE: Rules = |_, _, _, _| {
    panic!("Rules NONE should not be applied");
};

#[cfg(test)]
mod tests {
    use super::*;

    fn have_yahtzee() -> ScoreCard {
        let mut have_yahtzee = [vec![false; US_LENGTH], vec![false; LS_LENGTH]];
        have_yahtzee[LS][YAHTZEE_INDEX] = true;
        have_yahtzee
    }

    fn test_generic_upper_section(rules: Rules, bonus: Score) {
        // Upper section should award points when available
        assert_eq!(rules(&have_yahtzee(), 1, 0, 0), (5, bonus));

        // Attempt bonus in upper section for wrong field, should not award points but give bonus
        assert_eq!(rules(&have_yahtzee(), 1, 0, 1), (0, bonus));
    }

    fn test_generic_lower_section(rules: Rules, bonus: Score) -> ScoreCard {
        // Attempt score in lower section when upper section is still available,
        // should not award points
        assert_eq!(rules(&have_yahtzee(), 1, 1, 0), (0, 0));

        // Lower section should award points when upper section is full
        let mut upper_section_used = have_yahtzee();
        upper_section_used[US][0] = true;
        assert_eq!(rules(&upper_section_used, 1, 1, 0), (5, bonus));

        // Return for use afterwards
        upper_section_used
    }

    #[test]
    fn test_forced_joker() {
        test_generic_upper_section(FORCED_JOKER, YAHTZEE_BONUS);
        let upper_section_used = test_generic_lower_section(FORCED_JOKER, YAHTZEE_BONUS);

        // should also work with bonus
        assert_eq!(
            FORCED_JOKER(&upper_section_used, 1, 1, 2),
            (FULL_HOUSE_SCORE, YAHTZEE_BONUS)
        );
    }

    #[test]
    fn test_free_joker() {
        test_generic_upper_section(FREE_JOKER, YAHTZEE_BONUS);

        // Lower section should award points even when upper section is still available
        assert_eq!(FREE_JOKER(&have_yahtzee(), 1, 1, 0), (5, YAHTZEE_BONUS));

        // should also work with bonus, but only when upper section is full
        let mut upper_section_used = have_yahtzee();
        upper_section_used[US][0] = true;
        assert_eq!(
            FREE_JOKER(&upper_section_used, 1, 1, 3),
            (SMALL_STRAIGHT_SCORE, YAHTZEE_BONUS)
        );

        // Attempt bonus in lower section when upper section is still available,
        // should not award points
        assert_eq!(FREE_JOKER(&have_yahtzee(), 1, 1, 2), (0, 0));
    }

    #[test]
    fn test_original() {
        // Upper section should not award bonus, but points
        assert_eq!(ORIGINAL(&have_yahtzee(), 1, 0, 0), (5, 0));

        // Lower section should award points
        assert_eq!(ORIGINAL(&have_yahtzee(), 1, 1, 0), (5, YAHTZEE_BONUS));

        // should also work with bonus
        assert_eq!(
            ORIGINAL(&have_yahtzee(), 1, 1, 4),
            (LARGE_STRAIGHT_SCORE, YAHTZEE_BONUS)
        );
    }

    #[test]
    fn test_kniffel() {
        test_generic_upper_section(KNIFFEL, YAHTZEE_SCORE);
        let upper_section_used = test_generic_lower_section(KNIFFEL, YAHTZEE_SCORE);

        // should not work with bonus
        assert_eq!(KNIFFEL(&upper_section_used, 1, 1, 2), (0, 0));

        // but should work with Chance
        assert_eq!(KNIFFEL(&upper_section_used, 1, 1, 6), (5, YAHTZEE_SCORE));
    }

    #[test]
    #[should_panic]
    fn test_none() {
        NONE(&mut have_yahtzee(), 1, 0, 0);
    }
}
