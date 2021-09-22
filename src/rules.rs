// to quiet warnings, TODO use
#![allow(dead_code)]

use crate::global::*;
use crate::hands;
use crate::yahtzee_bonus_rules as bonus;

/// Rules for dice used
#[derive(Debug, PartialEq)]
pub struct DiceRules {
    /// Short name for caching
    pub short_name: char,
    /// Actual rules
    /// * Minimum and maximum pip, e.g. (1, 6) for d6
    /// * Frequency, e.g. 5 for key 6 in regular Yahtzee (5 d6)
    pub dice: Vec<(Die, Frequency)>,
}
/// Rules for reroll chips used, specify amount per player
type ChipsRules = Chips;
/// Function that calculates a score from a hand
type ScoreFunction = fn(&HandSlice) -> Score;
/// Rule for field on score card
pub struct SectionRule {
    /// Name of field for user interaction
    pub name: String,
    /// Function from hand to score
    pub function: ScoreFunction,
}
/// Rules in a section
type SectionRules = Vec<SectionRule>;
/// Rules for allowed fields (upper and lower section)
type FieldsRules = [SectionRules; 2];
/// Rule for upper section bonus
#[derive(Debug, PartialEq)]
pub struct USBonusRules {
    /// Score required to receive upper section bonus (63 in regular Yahtzee)
    pub threshold: Score,
    /// Bonus score granted when requirement was met
    pub bonus: Score,
}

/// Rules for a game
pub struct Rules {
    pub short_name: char,
    pub dice: DiceRules,
    pub chips: ChipsRules,
    pub fields: FieldsRules,
    pub us_bonus: USBonusRules,
    pub yahtzee_bonus: bonus::Rules,
}

/// Build upper section fields rules
fn build_upper_section_rules() -> SectionRules {
    // Manual unroll because a closure would capture the running variable and could thus not be
    // coerced into a function pointer (and that is with `move`).
    // If you are reading this and know of a better way, please tell me.

    vec![
        SectionRule {
            name: String::from("Count and Add Only Aces"),
            function: |hand| hands::generic_upper_section(1, hand),
        },
        SectionRule {
            name: String::from("Count and Add Only Twos"),
            function: |hand| hands::generic_upper_section(2, hand),
        },
        SectionRule {
            name: String::from("Count and Add Only Threes"),
            function: |hand| hands::generic_upper_section(3, hand),
        },
        SectionRule {
            name: String::from("Count and Add Only Fours"),
            function: |hand| hands::generic_upper_section(4, hand),
        },
        SectionRule {
            name: String::from("Count and Add Only Fives"),
            function: |hand| hands::generic_upper_section(5, hand),
        },
        SectionRule {
            name: String::from("Count and Add Only Sixes"),
            function: |hand| hands::generic_upper_section(6, hand),
        },
    ]
}

/// Build lower section fields rules
/// # Arguments
/// * `extreme` - build for Extreme variant
fn build_lower_section_rules(extreme: bool) -> SectionRules {
    let mut ls_fields_rules: SectionRules = vec![
        SectionRule {
            name: String::from("Three of a Kind"),
            function: |hand| hands::generic_identical(vec![3], hands::total, hand),
        },
        SectionRule {
            name: String::from("Four of a Kind"),
            function: |hand| hands::generic_identical(vec![4], hands::total, hand),
        },
    ];
    if extreme {
        ls_fields_rules.push(SectionRule {
            name: String::from("Two Pairs"),
            function: |hand| hands::generic_identical(vec![2, 2], hands::total, hand),
        });
        ls_fields_rules.push(SectionRule {
            name: String::from("Three Pairs"),
            function: |hand| hands::generic_identical(vec![2, 2, 2], |_| 35, hand),
        });
        ls_fields_rules.push(SectionRule {
            name: String::from("Two Triples"),
            function: |hand| hands::generic_identical(vec![3, 3], |_| 45, hand),
        });
    }

    ls_fields_rules.push(SectionRule {
        name: String::from("Full House"),
        function: |hand| hands::generic_identical(vec![2, 3], |_| FULL_HOUSE_SCORE, hand),
    });
    if extreme {
        ls_fields_rules.push(SectionRule {
            name: String::from("Grand Full House"),
            function: |hand| hands::generic_identical(vec![2, 4], |_| 45, hand),
        });
    }

    ls_fields_rules.push(SectionRule {
        name: String::from("Small Straight"),
        function: |hand| hands::generic_straight(4, SMALL_STRAIGHT_SCORE, hand),
    });
    ls_fields_rules.push(SectionRule {
        name: String::from("Large Straight"),
        function: |hand| hands::generic_straight(5, LARGE_STRAIGHT_SCORE, hand),
    });
    if extreme {
        ls_fields_rules.push(SectionRule {
            name: String::from("Highway"),
            function: |hand| hands::generic_straight(6, 50, hand),
        });
    }

    ls_fields_rules.push(SectionRule {
        name: String::from("Yahtzee"),
        function: |hand| hands::generic_identical(vec![5], |_| YAHTZEE_SCORE, hand),
    });
    if extreme {
        ls_fields_rules.push(SectionRule {
            name: String::from("Yahtzee Extreme"),
            function: |hand| hands::generic_identical(vec![6], |_| 75, hand),
        });
        ls_fields_rules.push(SectionRule {
            name: String::from("10 or less"),
            function: |hand| if hands::total(hand) <= 10 { 40 } else { 0 },
        });
        ls_fields_rules.push(SectionRule {
            name: String::from("33 or more"),
            function: |hand| if hands::total(hand) >= 33 { 40 } else { 0 },
        });
    }

    ls_fields_rules.push(SectionRule {
        name: String::from("Chance"),
        function: hands::total,
    });
    if extreme {
        ls_fields_rules.push(SectionRule {
            name: String::from("Super Chance"),
            function: |hand| 2 * hands::total(hand),
        });
    }

    ls_fields_rules
}

/// Build rules for Yahtzee
/// # Arguments
/// * `extreme` - build for Extreme variant
pub fn build_rules(extreme: bool, yahtzee_bonus: bonus::Rules) -> Rules {
    if extreme && yahtzee_bonus.short_name != bonus::NONE.short_name {
        panic!("Yahtzee Extreme with non-null bonus rules is undefined");
    }

    let short_name = match extreme {
        false => yahtzee_bonus.short_name,
        true => 'f',
    };

    // Five d6
    let mut dice = DiceRules {
        short_name: 'a',
        dice: vec![((1, 6), 5)],
    };
    if extreme {
        dice.short_name = 'b';
        // One d10, starting at 0
        dice.dice.push(((0, 9), 1));
    }
    let chips = if extreme { 3 } else { 0 };

    let us_fields_rules = build_upper_section_rules();
    let ls_fields_rules = build_lower_section_rules(extreme);

    let us_bonus = match extreme {
        true => USBonusRules {
            threshold: 73,
            bonus: 45,
        },
        _ => USBonusRules {
            threshold: 63,
            bonus: 35,
        },
    };

    Rules {
        short_name,
        dice,
        chips,
        fields: [us_fields_rules, ls_fields_rules],
        us_bonus,
        yahtzee_bonus,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_rules() {
        for (short_name, bonus_rules) in [
            ('a', bonus::FORCED_JOKER),
            ('b', bonus::FREE_JOKER),
            ('c', bonus::ORIGINAL),
            ('d', bonus::KNIFFEL),
            ('e', bonus::NONE),
        ] {
            let rules = build_rules(false, bonus_rules);

            assert_eq!(rules.short_name, short_name);
            assert_eq!(
                rules.dice,
                DiceRules {
                    short_name: 'a',
                    dice: vec![((1, 6), 5)]
                }
            );
            assert_eq!(rules.chips, 0);

            assert_eq!(rules.fields[US].len(), US_LENGTH);
            assert_eq!(rules.fields[LS].len(), LS_LENGTH);
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
                    assert_eq!((rules.fields[i][j].function)(hand), *score);
                }
            }

            assert_eq!(
                rules.us_bonus,
                USBonusRules {
                    threshold: 63,
                    bonus: 35,
                }
            );
            assert_eq!(rules.yahtzee_bonus.short_name, short_name);
        }
    }

    #[test]
    fn test_extreme_rules() {
        let rules = build_rules(true, bonus::NONE);

        assert_eq!(rules.short_name, 'f');
        assert_eq!(
            rules.dice,
            DiceRules {
                short_name: 'b',
                dice: vec![((1, 6), 5), ((0, 9), 1)],
            },
        );
        assert_eq!(rules.chips, 3);

        assert_eq!(rules.fields[US].len(), US_LENGTH);
        assert_eq!(rules.fields[LS].len(), 16);
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
                assert_eq!((rules.fields[i][j].function)(hand), *score);
            }
        }

        assert_eq!(
            rules.us_bonus,
            USBonusRules {
                threshold: 73,
                bonus: 45
            }
        );
        assert_eq!(rules.yahtzee_bonus.short_name, bonus::NONE.short_name);
    }
}
