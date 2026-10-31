//! `mechanics.json`: what a person checked against the rules text for records whose effects
//! are only prose. Each entry quotes the text it comes from, and a test holds that quote to
//! the data, so regenerating the SRD can't leave an entry describing rules that changed.

use crate::character::Recharge;
use crate::effects::{Action, Effect, Modifier, RollEffect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

/// A choice the player makes, filling `{choice}` in the entry's text fields.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub what: String,
    pub allowed: Vec<String>,
}

/// One version of an item whose versions are only listed in a table.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub(super) struct Tier {
    pub variant: String,
    pub name: String,
    pub roll: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub(super) struct Mechanics {
    /// Text from the record that this entry rests on.
    pub quote: String,
    pub modifiers: Vec<Modifier>,
    pub actions: Vec<Action>,
    pub choice: Option<Choice>,
    /// The `+N` of a variant is the item's bonus.
    pub variant_bonus: bool,
    pub tiers: Vec<Tier>,
    // Items.
    pub magic_bonus: i32,
    pub ac_bonus: i32,
    pub damage_type: String,
    /// The weapon an item not typed as one can be wielded as.
    pub as_weapon: String,
    pub charges_max: u32,
    pub recharge: Recharge,
    pub regain: String,
    // Spells, and the potions of `tiers`.
    pub roll: String,
    pub roll_formula: String,
    pub roll_effect: RollEffect,
    pub effect: Option<Effect>,
    // Features.
    pub uses_max: u32,
    pub uses_formula: String,
}

pub(super) fn all() -> &'static HashMap<String, Mechanics> {
    static ALL: OnceLock<HashMap<String, Mechanics>> = OnceLock::new();
    ALL.get_or_init(|| {
        serde_json::from_str(include_str!("mechanics.json")).expect("mechanics.json is valid")
    })
}

pub(super) fn of(id: &str) -> Option<&'static Mechanics> {
    all().get(id)
}

#[cfg(test)]
mod tests {
    use super::super::record;
    use super::*;

    #[test]
    fn every_quote_is_in_its_record() {
        let mut wrong = Vec::new();
        for (id, m) in all() {
            let Some(rec) = record(id) else {
                wrong.push(format!("{id}: no such record"));
                continue;
            };
            assert!(!m.quote.is_empty(), "{id} has no quote");
            if !rec["markdown"].as_str().unwrap().contains(&m.quote) {
                wrong.push(format!("{id}: {}", m.quote));
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }
}
