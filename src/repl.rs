use crate::global::*;
use crate::view_model::{Recommendation, ViewModel};
use crate::yahtzee_bonus_rules as bonus;

use core::num::ParseIntError;
use std::io;

use anyhow::{anyhow, Result};
use linefeed::{Interface, ReadResult};

pub fn run(mut view_model: ViewModel) -> io::Result<()> {
    let reader = Interface::new("")?;
    reader.set_prompt(">>> ")?;
    while let ReadResult::Input(input) = reader.read_line()? {
        println!(
            "{}",
            match match input.as_str() {
                "state" => output_state(&view_model),
                _ => recommend(&mut view_model, &input),
            } {
                Ok(out) => out,
                Err(err) => format!("Error: {}", err),
            }
        );
    }
    Ok(())
}

fn output_state(view_model: &ViewModel) -> Result<String> {
    let rules = &view_model.rules;
    let state = &view_model.state;
    Ok(format!(
        "{}{}{}{}",
        format!(
            "You have scored {} in the upper section and {} in the lower section.",
            state.score[0], state.score[1],
        ),
        rules
            .fields
            .iter()
            .zip(state.used.iter())
            .map(
                |(rules, useds)| rules.iter().zip(useds.iter()).map(|(rule, &used)| format!(
                    "{}: {}",
                    rule.name,
                    if used { "used" } else { "unused" },
                ))
            )
            .flatten()
            .fold(String::new(), |acc, info| format!("{}\n{}", acc, info)),
        match rules.yahtzee_bonus == bonus::NONE {
            true => String::new(),
            _ => format!(
                "\nYou have {}scored a Yahtzee.",
                if !state.scored_yahtzee { "not " } else { "" },
            ),
        },
        match rules.chips == 0 {
            true => String::new(),
            _ => format!("\nYou have {} chip(s) left.", state.chips),
        }
    ))
}

fn recommend(view_model: &mut ViewModel, input: &str) -> Result<String> {
    let mut split = input.split(' ');
    let d6_chars = split.next().unwrap().chars();
    let d6s = d6_chars
        .map(|c| c.to_string().parse().map_err(|e: ParseIntError| anyhow!(e)))
        .collect::<Result<Vec<_>>>()?;
    let mut partial_hand: PartialHand =
        PartialHand(d6s.iter().map(|&p| (D6, p)).collect::<Vec<_>>());
    if let Some(d10) = split.next() {
        partial_hand.0.push((D10, d10.parse()?));
    }
    Ok(match view_model.recommend(partial_hand)? {
        Recommendation::Reroll(partial_hand) => {
            let mut iter = partial_hand.0.iter().peekable();
            let mut out = String::new();
            if let Some(peek) = iter.peek() {
                if peek.0 == D10 {
                    out = String::from("You should keep the d10");
                    iter.next();
                }
            }
            let d6s = iter.map(|(_, pip)| pip.to_string()).collect::<Vec<_>>();
            if d6s.is_empty() {
                if out.is_empty() {
                    out = String::from("You should reroll altogether");
                }
            } else {
                let d6 = &d6s[..].join(", ");
                let recomm = match out.as_str() {
                    "" => format!("You should keep d6 {}", d6),
                    _ => format!(" and d6 {}", d6),
                };
                out += &recomm;
            }
            out.push('.');
            out
        }
        Recommendation::Field(section, field) => {
            format!(
                "You should score as {}.",
                view_model.rules.fields[section][field].name
            )
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::rules;
    use crate::strategy;

    #[test]
    fn test_output_state() {
        let mut rules = strategy::tests::very_simple_rules();
        let mut state = strategy::tests::very_simple_state();

        rules.fields[0] = vec![
            rules::SectionRule {
                name: String::from("Unwinnable 1"),
                function: |_| 0,
            },
            rules::SectionRule {
                name: String::from("Unwinnable 2"),
                function: |_| 0,
            },
        ];
        rules.yahtzee_bonus = bonus::FORCED_JOKER;

        state.used[0] = vec![true, false];

        let expected_fmt = "You have scored 0 in the upper section and 0 in the lower section.
Unwinnable 1: used
Unwinnable 2: unused
Throw 2: unused
You have {}scored a Yahtzee.
You have 2 chip(s) left.";

        let mut view_model = ViewModel {
            rules,
            state,
            rerolls: 0,
        };
        assert_eq!(
            output_state(&view_model).unwrap(),
            expected_fmt.replace("{}", "not "),
        );
        view_model.state.scored_yahtzee = true;
        assert_eq!(
            output_state(&view_model).unwrap(),
            expected_fmt.replace("{}", ""),
        );
    }

    #[test]
    fn test_recommend() {
        let rules = rules::build_rules(true, bonus::NONE);
        let mut state = strategy::State::new_from_rules(&rules);
        state.used = [
            vec![true; rules.fields[US].len()],
            [[true].repeat(rules.fields[LS].len() - 1), vec![false]].concat(),
        ];
        state.chips = 0;
        let view_model = ViewModel {
            rules,
            state,
            rerolls: 1,
        };

        assert_eq!(
            recommend(&mut view_model.clone(), "11111 0").unwrap(),
            String::from("You should reroll altogether.")
        );
        assert_eq!(
            recommend(&mut view_model.clone(), "11611 0").unwrap(),
            String::from("You should keep d6 6.")
        );
        assert_eq!(
            recommend(&mut view_model.clone(), "11111 9").unwrap(),
            String::from("You should keep the d10.")
        );
        assert_eq!(
            recommend(&mut view_model.clone(), "61116 9").unwrap(),
            String::from("You should keep the d10 and d6 6, 6.")
        );

        // XXX It would be cleaner to test this and the sorting of pips from ViewModel, but with
        // the current design, this is quite some fewer LOC. Might be refactored.
        assert!(recommend(&mut view_model.clone(), "11111").is_err());
        assert!(recommend(&mut view_model.clone(), "not numbers").is_err());
    }
}
