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

/// No Yahtzee bonus, Yahtzee extreme
pub const NONE: Rules = |_, _, _, _| {
    panic!("Rules NONE should not be applied");
};

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! have_yahtzee {
        () => {{
            let mut have_yahtzee = [vec![-1; US_LENGTH], vec![-1; LS_LENGTH]];
            have_yahtzee[LS][YAHTZEE_INDEX] = YAHTZEE_SCORE;
            have_yahtzee
        }};
    }

    fn test_generic_upper_section(rules: Rules) {
        let have_yahtzee = have_yahtzee!();

        // Upper section should award points when available
        let mut upper_section_available = have_yahtzee.clone();
        let mut expected = upper_section_available.clone();
        expected[US][0] = 5;
        expected[LS][YAHTZEE_INDEX] += YAHTZEE_BONUS;
        rules(&mut upper_section_available, 1, 0, 0);
        assert_eq!(upper_section_available, expected);

        // Attempt bonus in upper section for wrong field, should not award points
        let mut wrong_field = have_yahtzee.clone();
        expected = wrong_field.clone();
        expected[US][1] = 0;
        rules(&mut wrong_field, 1, 0, 1);
        assert_eq!(wrong_field, expected);
    }

    #[test]
    fn test_forced_joker() {
        test_generic_upper_section(FORCED_JOKER);
        let have_yahtzee = have_yahtzee!();

        // Attempt score in lower section when upper section is still available,
        // should not award points
        let mut upper_section_unused = have_yahtzee.clone();
        let mut expected = upper_section_unused.clone();
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

    #[test]
    fn test_free_joker() {
        test_generic_upper_section(FREE_JOKER);
        let have_yahtzee = have_yahtzee!();

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
        expected[LS][3] = 30;
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
    #[should_panic]
    fn test_none() {
        NONE(&mut have_yahtzee!(), 1, 0, 0);
    }
}

// TODO original, Kniffel
