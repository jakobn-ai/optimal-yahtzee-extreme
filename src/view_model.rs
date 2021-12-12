use crate::global::*;
use crate::rules;
use crate::strategy;

use anyhow::{anyhow, Result};

/// Recommendation for player, can be to...
#[derive(Debug, PartialEq)]
pub enum Recommendation {
    /// ...reroll a specific hand, or
    Reroll(PartialHand),
    /// ...choose a field
    Field(Section, Field),
}

/// Stateful ViewModel to adapt strategy and user interfaces
#[derive(Debug, Clone, PartialEq)]
pub struct ViewModel {
    /// Rules used for this game
    pub rules: rules::Rules,
    /// Strategy state player is in
    pub state: strategy::State,
    /// Rerolls in ongoing turn
    rerolls: Rerolls,
}

impl ViewModel {
    /// Recommend an action
    /// # Arguments
    /// * `hand` - base recommendation on this hand, _not_ assumed to be sorted
    /// # Returns
    /// Recommendation - see architecture of structure above
    pub fn recommend(&mut self, mut hand: PartialHand) -> Result<Recommendation> {
        hand.0.sort_unstable_by_key(|&(_, pip)| pip);
        hand.0.sort_by_key(|&(die, _)| die);

        // This check is also done in `strategy::probability_to_roll`, but it panics instead of
        // returning a Result to make caching and propagation easier
        let dice_rules = &self.rules.dice.dice;
        // TODO this does not check _which_ type of dice they are
        if !hand.is_full_hand(dice_rules) {
            return Err(anyhow!("Hand does not match selected rules"));
        }

        let reroll_recomm = strategy::choose_reroll(&self.state, &hand, self.rerolls, &self.rules);
        if reroll_recomm.hand.is_full_hand(dice_rules) {
            let field_recomm = strategy::choose_field(&self.state, &hand, &self.rules);
            self.state = field_recomm.state;
            self.rerolls = THROWS;
            return Ok(Recommendation::Field(
                field_recomm.section,
                field_recomm.field,
            ));
        }
        self.state = reroll_recomm.state;
        self.rerolls -= 1;
        Ok(Recommendation::Reroll(reroll_recomm.hand))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommend() {
        let rules = strategy::tests::very_simple_rules();
        let state = strategy::tests::very_simple_state();

        let mut view_model = ViewModel {
            rules,
            state: state.clone(),
            rerolls: 1,
        };

        // This hand does not win points
        let hand = PartialHand(vec![((1, 2), 1)]);

        // TODO test ordering
        // TODO test bad hand

        let mut expected_view_model_after_reroll = view_model.clone();
        let mut recommendation = view_model.recommend(hand.clone());
        let expected_recommendation = Recommendation::Reroll(PartialHand(Vec::new()));
        // We should reroll
        assert_eq!(recommendation, expected_recommendation);
        expected_view_model_after_reroll.rerolls = 0;
        assert_eq!(view_model, expected_view_model_after_reroll);

        let mut expected_state_after_finish = state;
        let mut expected_view_model_after_finish = view_model.clone();

        recommendation = view_model.recommend(hand);
        // We must use a field now
        assert_eq!(recommendation, Recommendation::Field(1, 0));
        expected_state_after_finish.used[1][0] = true;
        expected_view_model_after_finish.state = expected_state_after_finish;
        expected_view_model_after_finish.rerolls = THROWS;
        assert_eq!(view_model, expected_view_model_after_finish);
    }
}
