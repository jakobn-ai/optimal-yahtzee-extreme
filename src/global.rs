/// Global types and constants
use serde::{Deserialize, Serialize};

/// Number on a die (1-6 for d6)
pub type Pip = u8;
/// Die with minimum and maximum, e.g. (1, 6) for d6
pub type Die = (Pip, Pip);
/// Dice rolled (5 for regular Yahtzee), assumed to be sorted
pub type Hand = Vec<Pip>;
/// Hand but it makes Clippy happy
pub type HandSlice = [Pip];
/// Absolute frequency of a pip in a hand
pub type Frequency = u8;
/// Score on the card, both individual hands and end results
pub type Score = u16;
/// Reroll chip count
pub type Chips = u8;
/// Rerolls in a turn (can be negative for chips usage)
pub type Rerolls = i8;
/// Score card for player, only indicating whether used or not
/// * Array: Upper section and lower section
/// * Inner vector: Individual scores per hand
pub type ScoreCard = [Vec<bool>; 2];
/// Section index in scorecard
pub type Section = usize;
/// Field index in section
pub type Field = usize;

/// Combination of dice
/// * Minimum and maximum pip, e.g. (1, 6) for d6
/// * Frequency, e.g. 5 for key 6 in regular Yahtzee (5 d6)
#[derive(Clone, Debug, PartialEq)]
pub struct Dice(pub Vec<(Die, Frequency)>);

/// Partial hand, specifying dice and pips
pub type PartialHandVec = Vec<(Die, Pip)>;

/// Wrap in struct for derives
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartialHand(pub PartialHandVec);

impl PartialHand {
    /// Compact format for cache keys
    pub fn compact_fmt(&self) -> String {
        self.0
            .iter()
            .map(|((min, max), pip)| format!("{},{},{}", min, max, pip))
            .reduce(|a, b| format!("{},{}", a, b))
            .unwrap_or_else(|| String::from(""))
    }

    /// Decide whether this is a full hand according to rules `dice`
    pub fn is_full_hand(&self, dice: &Dice) -> bool {
        dice.0.iter().all(|&(rules_die, freq)| {
            let dice = self.0.iter().filter(|(hand_die, _)| hand_die == &rules_die);
            dice.count() as Frequency == freq
        })
    }

    /// Decide whether this has the length of a full hand according to rules `dice`
    pub fn has_full_hand_length(&self, dice: &Dice) -> bool {
        self.0.len() == dice.0.iter().map(|(_, freq)| freq).sum::<Frequency>() as usize
    }
}

/// Tuple for d6 pips
pub const D6: (Pip, Pip) = (1, 6);
/// Tuple for d10 pips
pub const D10: (Pip, Pip) = (0, 9);

/// Permitted rerolls per round
pub const REROLLS: i8 = 2;
/// Index of upper section
pub const US: usize = 0;
/// Index of lower section
pub const LS: usize = 1;
/// Size of a Yahtzee
pub const YAHTZEE_SIZE: Frequency = 5;
/// Index of Yahtzee in lower section (zero-indexed)
pub const YAHTZEE_INDEX: usize = 5;

/// Scores of various hands
pub const FULL_HOUSE_SCORE: Score = 25;
pub const SMALL_STRAIGHT_SCORE: Score = 30;
pub const LARGE_STRAIGHT_SCORE: Score = 40;
pub const YAHTZEE_SCORE: Score = 50;

/// Field count in upper section
#[cfg(test)]
pub const US_LENGTH: usize = 6;
/// Field count in lower section (regular only)
#[cfg(test)]
pub const LS_LENGTH: usize = 7;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_fmt_partial_hand() {
        assert_eq!(PartialHand(Vec::new()).compact_fmt(), "");
        assert_eq!(
            PartialHand(vec![(D6, 1), (D10, 2)]).compact_fmt(),
            "1,6,1,0,9,2"
        );
    }

    #[test]
    fn test_is_full_hand() {
        let hand = PartialHand(vec![(D6, 1), (D10, 2)]);
        assert!(hand.is_full_hand(&Dice(vec![(D6, 1), (D10, 1)])));
        assert!(!hand.is_full_hand(&Dice(vec![(D6, 2)])));
    }

    #[test]
    fn test_has_full_hand_length() {
        let hand = PartialHand(vec![(D6, 1)]);
        assert!(hand.has_full_hand_length(&Dice(vec![(D6, 1)])));
        assert!(!hand.has_full_hand_length(&Dice(vec![(D6, 2)])));
    }
}
