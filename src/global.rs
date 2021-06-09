/// Global types and constants

/// Number on a die (1-6 for d6)
pub type Pip = u8;
/// Absolute frequency of a pip in a hand
pub type Frequency = u8;
/// Score on the card, both individual hands and end results
pub type Score = i16;
/// Scorecard for player
/// * Array: Upper section and lower section
/// * Inner vector: Individual scores per&hand
pub type ScoreCard = [Vec<Score>; 2];

/// Index of upper section
pub const US: usize = 0;
/// Field count in upper section
pub const US_LENGTH: usize = 6;
/// Index of lower section
pub const LS: usize = 1;
/// Field count in lower section (regular only)
pub const LS_LENGTH: usize = 7;
/// Size of a Yahtzee
pub const YAHTZEE_SIZE: Frequency = 5;
/// Index of Yahtzee in lower section (zero-indexed)
pub const YAHTZEE_INDEX: usize = 5;
/// Score of a Yahtzee
pub const YAHTZEE_SCORE: Score = 50;
