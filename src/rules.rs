// TODO format
// to quiet warnings, TODO use
#![allow(dead_code)]

use std::collections::HashMap;

use crate::hands::*;

/// Scorecard for player
/// * Array: Upper section and lower section
/// * Inner vector: Individual scores per hand
pub type ScoreCard = [Vec<Score>; 2];

/// Rules for dice used
/// * Key: Minimum and maximum pip, e.g. (1, 6) for d6
/// * Value: Frequency, e.g. 5 for key 6 in regular Yahtzee (5 d6)
type DiceRules = HashMap<(Pip, Pip), Frequency>;
/// Rules for reroll chips used, specify amount per player
type ChipsRules = u8;
/// Rules for allowed fields (upper or lower section)
/// * First field: Name of field for user interaction
/// * Second field: Function from hand to score
type SectionRules = Vec<(String, Box<dyn Fn(Hand) -> Score>)>;
/// Rules for allowed fields (upper and lower section)
type FieldsRules = [SectionRules; 2];
/// Rule for upper section bonus
/// * First field: Score required to receive upper section bonus (63 in regular Yahtzee)
/// * Second field: Bonus score granted when requirement was met
type USBonusRules = [Score; 2];
/// Rules for Yahtzee bonus
/// # Arguments
/// * Scorecard to check for eligibility
/// * Pip that the Yahtzee was (e.g. a Yahtzee of 5s) to check for eligibility
/// # Returns
/// * Possible scorecards after applying bonus
pub type YahtzeeBonusRules = fn(ScoreCard, Pip) -> Vec<ScoreCard>;

/// Rules for a game
struct Rules {
    dice: DiceRules,
    chips: ChipsRules,
    fields: FieldsRules,
    us_bonus: USBonusRules,
    yahtzee_bonus: YahtzeeBonusRules,
}

/// Build upper section fields rules
fn build_upper_section_rules() -> SectionRules {
    let upper_section_names = ["Aces", "Twos", "Threes", "Fours", "Fives", "Sixes"]
        .iter()
        .map(|field| format!("Count and Add Only {}", field));
    // Curry fields 1-6 into generic_upper_section to get actual upper section fields rules
    let upper_section_functions = (1..7).map(|field: Pip| -> Box<dyn Fn(Hand) -> Score> {
        Box::new(move |hand| generic_upper_section(field, hand))
    });
    upper_section_names.zip(upper_section_functions).collect()
}

/// Build lower section fields rules
/// # Arguments
/// * `extreme` - build for Extreme variant
fn build_lower_section_rules(extreme: bool) -> SectionRules {
    // Curry lower section fields requirements into generic_identical/generic_straight
    // I cannot rid the feeling that this should be possible without calling `String::from` for
    // every field name, but I have trouble mapping over this `Vec` afterwards because `Box` does
    // not implement `Copy` and I could just `zip` as in `build_upper_section_rules`, but that
    // would eliminate the comment function of these strings.
    // If you are reading this and know of a better way, please tell me.
    let mut ls_fields_rules: SectionRules = vec![
        (
            String::from("Three of a Kind"),
            Box::new(|hand| generic_identical(vec![3], total, hand)),
        ),
        (
            String::from("Four of a Kind"),
            Box::new(|hand| generic_identical(vec![4], total, hand)),
        ),
    ];
    if extreme {
        ls_fields_rules.push((
            String::from("Two Pairs"),
            Box::new(|hand| generic_identical(vec![2, 2], total, hand)),
        ));
        ls_fields_rules.push((
            String::from("Three Pairs"),
            Box::new(|hand| generic_identical(vec![2, 2, 2], |_| 35, hand)),
        ));
        ls_fields_rules.push((
            String::from("Two Triples"),
            Box::new(|hand| generic_identical(vec![3, 3], |_| 45, hand)),
        ));
    }

    ls_fields_rules.push((
        String::from("Full House"),
        Box::new(|hand| generic_identical(vec![2, 3], |_| 25, hand)),
    ));
    if extreme {
        ls_fields_rules.push((
            String::from("Grand Full House"),
            Box::new(|hand| generic_identical(vec![2, 4], |_| 45, hand)),
        ));
    }

    ls_fields_rules.push((
        String::from("Small Straight"),
        Box::new(|hand| generic_straight(4, 30, hand)),
    ));
    ls_fields_rules.push((
        String::from("Large Straight"),
        Box::new(|hand| generic_straight(5, 40, hand)),
    ));
    if extreme {
        ls_fields_rules.push((
            String::from("Highway"),
            Box::new(|hand| generic_straight(6, 50, hand)),
        ));
    }

    ls_fields_rules.push((
        String::from("Yahtzee"),
        Box::new(|hand| generic_identical(vec![5], |_| 50, hand)),
    ));
    if extreme {
        ls_fields_rules.push((
            String::from("Yahtzee Extreme"),
            Box::new(|hand| generic_identical(vec![6], |_| 75, hand)),
        ));
        ls_fields_rules.push((
            String::from("10 or less"),
            Box::new(|hand| if total(hand) <= 10 { 40 } else { 0 }),
        ));
        ls_fields_rules.push((
            String::from("33 or more"),
            Box::new(|hand| if total(hand) >= 33 { 40 } else { 0 }),
        ));
    }

    ls_fields_rules.push((String::from("Chance"), Box::new(total)));
    if extreme {
        ls_fields_rules.push((
            String::from("Super Chance"),
            Box::new(|hand| 2 * total(hand)),
        ));
    }

    ls_fields_rules
}

/// Build rules for Yahtzee
/// # Arguments
/// * `extreme` - build for Extreme variant
fn build_rules(extreme: bool) -> Rules {
    // Five d6
    let mut dice_rules: DiceRules = [((1, 6), 5)].iter().cloned().collect();
    if extreme {
        // One d10, starting at 0
        dice_rules.insert((0, 9), 1);
    }
    let chips_rules = if extreme { 3 } else { 0 };

    let us_fields_rules = build_upper_section_rules();
    let ls_fields_rules = build_lower_section_rules(extreme);

    let us_bonus_rules = if extreme { [73, 45] } else { [63, 35] };
    // TODO this is not valid for regular Yahtzee -- add rules
    let yahtzee_rules = |_, _| vec![];

    Rules {
        dice: dice_rules,
        chips: chips_rules,
        fields: [us_fields_rules, ls_fields_rules],
        us_bonus: us_bonus_rules,
        yahtzee_bonus: yahtzee_rules,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_rules() {
        let rules = build_rules(false);

        assert_eq!(rules.dice, [((1, 6), 5)].iter().cloned().collect());
        assert_eq!(rules.chips, 0);

        assert_eq!(rules.fields[0].len(), 6);
        assert_eq!(rules.fields[1].len(), 7);
        for (i, field) in [
            vec![
                (vec![1, 1, 1, 1, 2], 4),
                (vec![1, 2, 2, 2, 2], 8),
                (vec![1, 3, 3, 3, 3], 12),
                (vec![1, 4, 4, 4, 4], 16),
                (vec![1, 5, 5, 5, 5], 20),
                (vec![1, 6, 6, 6, 6], 24),
            ],
            vec![
                (vec![1, 1, 1, 2, 3], 8),
                (vec![1, 1, 1, 1, 2], 6),
                (vec![1, 1, 1, 2, 2], 25),
                (vec![1, 2, 3, 4, 6], 30),
                (vec![1, 2, 3, 4, 5], 40),
                (vec![1, 1, 1, 1, 1], 50),
                (vec![1, 1, 1, 1, 2], 6),
            ],
        ]
        .iter()
        .enumerate()
        {
            for (j, (hand, score)) in field.iter().enumerate() {
                assert_eq!(rules.fields[i][j].1(hand.to_owned()), *score);
            }
        }

        assert_eq!(rules.us_bonus, [63, 35]);
    }

    #[test]
    fn test_extreme_rules() {
        let rules = build_rules(true);

        assert_eq!(
            rules.dice,
            [((1, 6), 5), ((0, 9), 1)].iter().cloned().collect()
        );
        assert_eq!(rules.chips, 3);

        assert_eq!(rules.fields[0].len(), 6);
        assert_eq!(rules.fields[1].len(), 16);
        for (i, field) in [
            vec![
                (vec![1, 1, 1, 1, 2, 2], 4),
                (vec![1, 1, 2, 2, 2, 2], 8),
                (vec![1, 1, 3, 3, 3, 3], 12),
                (vec![1, 1, 4, 4, 4, 4], 16),
                (vec![1, 1, 5, 5, 5, 5], 20),
                (vec![1, 1, 6, 6, 6, 6], 24),
            ],
            vec![
                (vec![1, 1, 1, 2, 3, 4], 12),
                (vec![1, 1, 1, 1, 2, 3], 9),
                (vec![1, 1, 2, 2, 3, 4], 13),
                (vec![1, 1, 2, 2, 3, 3], 35),
                (vec![1, 1, 1, 2, 2, 2], 45),
                (vec![1, 1, 1, 2, 2, 3], 25),
                (vec![1, 1, 1, 1, 2, 2], 45),
                (vec![1, 1, 2, 2, 3, 4], 30),
                (vec![1, 1, 2, 3, 4, 5], 40),
                (vec![1, 2, 3, 4, 5, 6], 50),
                (vec![1, 1, 1, 1, 1, 2], 50),
                (vec![1, 1, 1, 1, 1, 1], 75),
                (vec![1, 1, 1, 2, 2, 3], 40),
                (vec![5, 5, 5, 5, 5, 8], 40),
                (vec![1, 1, 1, 1, 2, 3], 9),
                (vec![1, 1, 1, 1, 2, 3], 18),
            ],
        ]
        .iter()
        .enumerate()
        {
            for (j, (hand, score)) in field.iter().enumerate() {
                assert_eq!(rules.fields[i][j].1(hand.to_owned()), *score);
            }
        }

        assert_eq!(rules.us_bonus, [73, 45]);

        let mut test_yahtzee_bonus_ls = vec![0; 16];
        // add a Yahtzee just to make sure
        test_yahtzee_bonus_ls[10] = 50;
        assert_eq!(
            (rules.yahtzee_bonus)([vec![0; 6], test_yahtzee_bonus_ls], 1),
            Vec::<ScoreCard>::new()
        );
    }
}
