// TODO use in REPL
#![allow(dead_code)]
use crate::global::*;
use crate::rules;
use crate::strategy;

/// Recommendation for player, can be to...
enum Recommendation {
    /// ...reroll a specific hand, or
    Reroll(PartialHand),
    /// ...choose a field
    Field(Section, Field),
}

/// ViewModel to adapt strategy and user interfaces
struct ViewModel {
    /// Strategy state player is in
    state: strategy::State,
    /// Rules used for this game
    rules: rules::Rules,
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
