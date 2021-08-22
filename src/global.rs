/// Global types and constants

/// Number on a die (1-6 for d6)
pub type Pip = u8;
/// Die with minimum and maximum, e.g. (1, 6) for d6
pub type Die = (Pip, Pip);
/// Dice rolled (5 for regular Yahtzee), assumed to be sorted
pub type HandSlice = [Pip];
/// Absolute frequency of a pip in a hand
pub type Frequency = u8;
/// Score on the card, both individual hands and end results
pub type Score = i16;
/// Reroll chip count
pub type Chips = u8;
/// Score card for player, only indicating whether used or not
/// * Array: Upper section and lower section
/// * Inner vector: Individual scores per hand
pub type ScoreCard = [Vec<bool>; 2];
/// Section index in scorecard
pub type Section = usize;
/// Field index in section
pub type Field = usize;

/// Index of upper section
pub const US: usize = 0;
/// Field count in upper section
pub const US_LENGTH: usize = 6;
/// Index of lower section
#[cfg(test)]
pub const LS: usize = 1;
/// Size of a Yahtzee
pub const YAHTZEE_SIZE: Frequency = 5;
/// Index of Yahtzee in lower section (zero-indexed)
#[cfg(test)]
pub const YAHTZEE_INDEX: usize = 5;

/// Scores of various hands
pub const FULL_HOUSE_SCORE: Score = 25;
pub const SMALL_STRAIGHT_SCORE: Score = 30;
pub const LARGE_STRAIGHT_SCORE: Score = 40;
pub const YAHTZEE_SCORE: Score = 50;

/// Field count in lower section (regular only)
#[cfg(test)]
pub const LS_LENGTH: usize = 7;
