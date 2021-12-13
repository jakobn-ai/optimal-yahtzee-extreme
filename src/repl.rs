use crate::global::*;
use crate::view_model::{Recommendation, ViewModel};
use crate::yahtzee_bonus_rules as bonus;

use std::io;

use anyhow::Result;
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
            "You have scored {} in the upper section and {} in the lower section.\n",
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
                if state.scored_yahtzee { "not " } else { "" },
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
    let d6s = split.next().unwrap().chars();
    let mut partial_hand: PartialHand = PartialHand(
        d6s.map(|c| Ok((D6, c.to_string().parse()?)))
            .collect::<Result<PartialHandVec>>()?,
    );
    if let Some(d10) = split.next() {
        partial_hand.0.push((D10, d10.parse()?));
    }
    Ok(match view_model.recommend(partial_hand)? {
        Recommendation::Reroll(partial_hand) => {
            let mut iter = partial_hand.0.iter();
            let d6s = iter
                .by_ref()
                .map_while(|&(die, pip)| match die {
                    D6 => Some(pip.to_string()),
                    _ => None,
                })
                .collect::<Vec<String>>();
            let mut out = String::new();
            if !d6s.is_empty() {
                out = format!("You should keep d6 {}", &d6s[..].join(", "));
            }
            if iter.next().is_some() {
                out += match out.as_str() {
                    "" => "You should keep the d10",
                    _ => " and the d10",
                }
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

// TODO tests
