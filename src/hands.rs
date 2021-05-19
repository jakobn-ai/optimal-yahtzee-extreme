use crate::global::*;

/// Dice rolled (5 for regular Yahtzee), assumed to be sorted
pub type Hand = [Pip];

/// Upper section fields
/// # Arguments
/// * `field` - required field, e.g. `3` for Count Threes
/// * `hand` - sorted
pub fn generic_upper_section(field: Pip, hand: &Hand) -> Score {
    hand.iter()
        .skip_while(|&&pip| pip != field)
        .take_while(|&&pip| pip == field)
        .count() as Score
        * field as Score
}

/// Calculate sum of hand
#[inline]
pub fn total(hand: &Hand) -> Score {
    hand.iter().sum::<Pip>() as Score
}

/// Frequency analysis over hand
/// # Arguments
/// * `hand` - sorted
/// # Returns
/// Vector of frequencies, sorted, without pips
fn identical(hand: &[Pip]) -> Vec<Frequency> {
    let mut groups = Vec::new();
    let mut iter = hand.iter().peekable();
    while let Some(pip) = iter.next() {
        let mut count = 1;
        while iter.peek().map(|&peek| pip == peek).unwrap_or(false) {
            iter.next();
            count += 1;
        }
        if count > 1 {
            groups.push(count);
        }
    }
    groups.sort_unstable();
    groups
}

/// Lower section fields based on having identical pips
/// e.g. Three of a Kind, Full House, Yahtzee
/// # Arguments
/// * `required` - sorted vector of required frequencies, e.g. `[2, 3]` for Full House
/// * `score` - function to calculate score based on hand,
///             e.g. `total` for Three of a Kind, `|_| 50` for Yahtzee
/// * `hand` - sorted
pub fn generic_identical(
    required: Vec<Frequency>,
    score: fn(&Hand) -> Score,
    hand: &Hand,
) -> Score {
    let groups = identical(&hand);
    let mut present = groups.iter();
    'next_req: for req in required.iter() {
        // Discard all non-matching present frequencies.
        // Note that the present frequency being greater than the required is also considered a
        // match, e.g. a Four of a Kind can also be counted as a Three of a Kind.
        // Note also that the required frequencies must be counted individually, e.g. a Yahtzee
        // cannot be counted as a Full House when ignoring bonus rules (the pips must be different).
        while let Some(freq) = present.next() {
            if freq >= req {
                // Search next criterion
                continue 'next_req;
            }
        }
        // Criteria were still left, but no more groups were present
        return 0;
    }
    score(hand)
}

/// Lower section straights
/// # Arguments
/// * `length` - desired length, e.g. `5` for Large Straight
/// * `score` - score if the hand is a straight, e.g. `40` for Large Straight
/// * `hand` - sorted
pub fn generic_straight(length: Frequency, score: Score, hand: &Hand) -> Score {
    let mut iter = hand.iter().peekable();
    while let Some(pip) = iter.next() {
        let mut count = 1;
        while let Some(&&peek) = iter.peek() {
            if peek <= pip + count {
                iter.next();
                // skip if it was just the same, not one more
                if peek == pip + count {
                    count += 1;
                }
            } else {
                break;
            }
        }
        if count >= length {
            return score;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_upper_section() {
        assert_eq!(generic_upper_section(1, &vec![1, 1, 1, 3, 5]), 3);
        assert_eq!(generic_upper_section(5, &vec![2, 5, 5, 5, 6]), 15);
        assert_eq!(generic_upper_section(6, &vec![3, 4, 6, 6, 6]), 18);
    }

    #[test]
    fn test_generic_identical() {
        assert_eq!(generic_identical(vec![3], total, &vec![1, 1, 2, 3, 5]), 0);
        assert_eq!(generic_identical(vec![3], total, &vec![1, 1, 1, 3, 5]), 11);
        assert_eq!(generic_identical(vec![3], total, &vec![2, 5, 5, 5, 6]), 23);
        assert_eq!(generic_identical(vec![3], total, &vec![3, 4, 6, 6, 6]), 25);
        assert_eq!(generic_identical(vec![3], total, &vec![3, 6, 6, 6, 6]), 27);

        assert_eq!(
            generic_identical(vec![2, 3], |_| 25, &vec![2, 2, 3, 3, 3]),
            25
        );
        assert_eq!(
            generic_identical(vec![2, 3], |_| 25, &vec![2, 2, 3, 3, 4]),
            0
        );
        assert_eq!(
            generic_identical(vec![2, 3], |_| 25, &vec![2, 2, 2, 2, 2]),
            0
        );
        assert_eq!(generic_identical(vec![5], |_| 50, &vec![2, 2, 2, 2, 2]), 50);
        assert_eq!(
            generic_identical(vec![2, 2, 2], |_| 45, &vec![2, 2, 4, 4, 6, 6]),
            45
        );
    }

    #[test]
    fn test_generic_straight() {
        assert_eq!(generic_straight(4, 30, &vec![1, 2, 2, 3, 4, 6]), 30);
        assert_eq!(generic_straight(4, 30, &vec![1, 2, 3, 4, 6, 7]), 30);
        assert_eq!(generic_straight(4, 30, &vec![1, 3, 4, 5, 6, 7]), 30);
        assert_eq!(generic_straight(4, 30, &vec![1, 2, 4, 5, 6, 7]), 30);
        assert_eq!(generic_straight(4, 30, &vec![1, 1, 2, 3, 6, 7]), 0);
        assert_eq!(generic_straight(4, 30, &vec![1, 3, 4, 5, 6, 7]), 30);
        assert_eq!(generic_straight(5, 40, &vec![1, 3, 4, 5, 6, 7]), 40);
    }
}
