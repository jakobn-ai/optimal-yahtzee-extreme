// TODO use in REPL
#![allow(dead_code)]
use crate::global::*;
use crate::rules;
use crate::strategy;

/// Recommendation for player, can be to...
#[derive(Debug, PartialEq)]
enum Recommendation {
    /// ...reroll a specific hand, or
    Reroll(PartialHand),
    /// ...choose a field
    Field(Section, Field),
}

/// Stateful ViewModel to adapt strategy and user interfaces
#[derive(Debug, Clone, PartialEq)]
struct ViewModel {
    /// Rules used for this game
    rules: rules::Rules,
    /// Strategy state player is in
    state: strategy::State,
    /// Rerolls in ongoing turn
    rerolls: Rerolls,
}

impl ViewModel {
    /// Recommend an action
    /// # Arguments
    /// * `hand` - base recommendation on this hand
    /// # Returns
    /// Recommendation - see architecture of structure above
    fn recommend(&mut self, hand: &PartialHand) -> Recommendation {
        let reroll_recomm = strategy::choose_reroll(&self.state, hand, self.rerolls, &self.rules);
        if reroll_recomm.hand.is_full_hand(&self.rules.dice.dice) {
            let field_recomm = strategy::choose_field(&self.state, hand, &self.rules);
            self.state = field_recomm.state;
            self.rerolls = THROWS;
            return Recommendation::Field(field_recomm.section, field_recomm.field);
        }
        self.state = reroll_recomm.state;
        self.rerolls -= 1;
        Recommendation::Reroll(reroll_recomm.hand)
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

        let mut expected_view_model_after_reroll = view_model.clone();
        let mut recommendation = view_model.recommend(&hand);
        let expected_recommendation = Recommendation::Reroll(PartialHand(Vec::new()));
        // We should reroll
        assert_eq!(recommendation, expected_recommendation);
        expected_view_model_after_reroll.rerolls = 0;
        assert_eq!(view_model, expected_view_model_after_reroll);

        let mut expected_state_after_finish = state;
        let mut expected_view_model_after_finish = view_model.clone();

        recommendation = view_model.recommend(&hand);
        // We must use a field now
        assert_eq!(recommendation, Recommendation::Field(1, 0));
        expected_state_after_finish.used[1][0] = true;
        expected_view_model_after_finish.state = expected_state_after_finish;
        expected_view_model_after_finish.rerolls = THROWS;
        assert_eq!(view_model, expected_view_model_after_finish);
    }
}
