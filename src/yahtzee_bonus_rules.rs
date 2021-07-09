// due to additional bonus rules, TODO use
#![allow(dead_code)]

use std::collections::HashMap;

use crate::global::*;

/// Rules for Yahtzee bonus
/// # Arguments
/// * Scorecard, is overwritten
/// * Pip that the Yahtzee was (e.g. a Yahtzee of Fives)
/// * Section where score is applied
/// * Field where score is applied within section (assumed to be free)
pub type Rules = fn(&mut ScoreCard, Pip, Section, Field);

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
                // Joker
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

/// Kniffel rules, as published in German-speaking countries
pub const KNIFFEL: Rules = |score_card, pip, section, field| {
    score_card[section][field] = match section {
        US => match pip {
            pip if pip as usize == field + 1 => (YAHTZEE_SIZE * pip) as Score,
            _ => 0,
        },
        _ => match score_card[US][pip as usize - 1] {
            -1 => 0,
            _ => match joker_fields().get(&field) {
                // No joker
                Some(_) => 0,
                None => (YAHTZEE_SIZE * pip) as Score,
            },
        },
    };
    if score_card[section][field] != 0 {
        score_card[LS][YAHTZEE_INDEX] += YAHTZEE_SCORE;
    }
};

/// Free Joker rules, a popular alternative
pub const FREE_JOKER: Rules = |score_card, pip, section, field| {
    score_card[section][field] = match section {
        US => match pip {
            pip if pip as usize == field + 1 => (YAHTZEE_SIZE * pip) as Score,
            _ => 0,
        },
        _ => match joker_fields().get(&field) {
            Some(&score) => match score_card[US][pip as usize - 1] {
                // Upper section unused, not allowed to use joker
                -1 => 0,
                _ => score,
            },
            None => (YAHTZEE_SIZE * pip) as Score,
        },
    };
    if score_card[section][field] != 0 {
        score_card[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
    }
};

/// Original 1956 rules
pub const ORIGINAL: Rules = |score_card, pip, section, field| {
    score_card[section][field] = match section {
        // Upper section cannot be used
        US => 0,
        _ => match joker_fields().get(&field) {
            Some(&score) => score,
            None => (YAHTZEE_SIZE * pip) as Score,
        },
    };
    if score_card[section][field] != 0 {
        score_card[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
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
        let mut have_yahtzee = [vec![-1; US_LENGTH], vec![-1; LS_LENGTH]];
        have_yahtzee[LS][YAHTZEE_INDEX] = YAHTZEE_SCORE;
        have_yahtzee
    }

    fn test_generic_upper_section(rules: Rules, bonus: Score) {
        let have_yahtzee = have_yahtzee();

        // Upper section should award points when available
        let mut upper_section_available = have_yahtzee.clone();
        let mut expected = upper_section_available.clone();
        expected[US][0] = 5;
        expected[LS][YAHTZEE_INDEX] += bonus;
        rules(&mut upper_section_available, 1, 0, 0);
        assert_eq!(upper_section_available, expected);

        // Attempt bonus in upper section for wrong field, should not award points
        let mut wrong_field = have_yahtzee.clone();
        expected = wrong_field.clone();
        expected[US][1] = 0;
        rules(&mut wrong_field, 1, 0, 1);
        assert_eq!(wrong_field, expected);
    }

    fn test_generic_lower_section(rules: Rules, bonus: Score) -> ScoreCard {
        let have_yahtzee = have_yahtzee();

        // Attempt score in lower section when upper section is still available,
        // should not award points
        let mut upper_section_unused = have_yahtzee.clone();
        let mut expected = upper_section_unused.clone();
        expected[LS][0] = 0;
        rules(&mut upper_section_unused, 1, 1, 0);
        assert_eq!(upper_section_unused, expected);

        // Lower section should award points when upper section is full
        let mut upper_section_used = have_yahtzee.clone();
        upper_section_used[US][0] = 0;
        expected = upper_section_used.clone();
        expected[LS][0] = 5;
        expected[LS][YAHTZEE_INDEX] += bonus;
        rules(&mut upper_section_used, 1, 1, 0);
        assert_eq!(upper_section_used, expected);

        // Return for use afterwards
        upper_section_used
    }

    #[test]
    fn test_forced_joker() {
        test_generic_upper_section(FORCED_JOKER, YAHTZEE_BONUS);
        let mut upper_section_used = test_generic_lower_section(FORCED_JOKER, YAHTZEE_BONUS);

        // should also work with bonus
        let mut expected = upper_section_used.clone();
        expected[LS][2] = FULL_HOUSE_SCORE;
        expected[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        FORCED_JOKER(&mut upper_section_used, 1, 1, 2);
        assert_eq!(upper_section_used, expected);
    }

    #[test]
    fn test_free_joker() {
        test_generic_upper_section(FREE_JOKER, YAHTZEE_BONUS);
        let have_yahtzee = have_yahtzee();

        // Lower section should award points even when upper section is still available
        let mut upper_section_unused = have_yahtzee.clone();
        let mut expected = upper_section_unused.clone();
        expected[LS][0] = 5;
        expected[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        FREE_JOKER(&mut upper_section_unused, 1, 1, 0);
        assert_eq!(upper_section_unused, expected);

        // should also work with bonus, but only when upper section is full
        let mut upper_section_used = upper_section_unused.clone();
        upper_section_used[US][0] = 1;
        expected = upper_section_used.clone();
        expected[LS][3] = SMALL_STRAIGHT_SCORE;
        expected[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        FREE_JOKER(&mut upper_section_used, 1, 1, 3);
        assert_eq!(upper_section_used, expected);

        // Attempt bonus in lower section when upper section is still available,
        // should not award points
        upper_section_unused = have_yahtzee.clone();
        expected = upper_section_unused.clone();
        expected[LS][2] = 0;
        FREE_JOKER(&mut upper_section_unused, 1, 1, 2);
        assert_eq!(upper_section_unused, expected);
    }

    #[test]
    fn test_original() {
        let have_yahtzee = have_yahtzee();

        // Upper section should not award points
        let mut upper_section = have_yahtzee.clone();
        let mut expected = upper_section.clone();
        expected[US][0] = 0;
        ORIGINAL(&mut upper_section, 1, 0, 0);
        assert_eq!(upper_section, expected);

        // Lower section should award points
        let mut lower_section = have_yahtzee.clone();
        expected = lower_section.clone();
        expected[LS][0] = 5;
        expected[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        ORIGINAL(&mut lower_section, 1, 1, 0);
        assert_eq!(lower_section, expected);

        // should also work with bonus
        expected[LS][4] = LARGE_STRAIGHT_SCORE;
        expected[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        ORIGINAL(&mut lower_section, 1, 1, 4);
        assert_eq!(lower_section, expected);
    }

    #[test]
    fn test_kniffel() {
        test_generic_upper_section(KNIFFEL, YAHTZEE_SCORE);
        let mut upper_section_used = test_generic_lower_section(KNIFFEL, YAHTZEE_SCORE);

        // should not work with bonus
        let mut expected = upper_section_used.clone();
        expected[LS][2] = 0;
        KNIFFEL(&mut upper_section_used, 1, 1, 2);
        assert_eq!(upper_section_used, expected);

        // but should work with Chance
        expected[LS][6] = 5;
        expected[LS][YAHTZEE_INDEX] += YAHTZEE_SCORE;
        KNIFFEL(&mut upper_section_used, 1, 1, 6);
        assert_eq!(upper_section_used, expected);
    }

    #[test]
    #[should_panic]
    fn test_none() {
        NONE(&mut have_yahtzee(), 1, 0, 0);
    }
}
