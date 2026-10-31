//! Modifiers, effects and actions: the parts of items, features and spells that change
//! numbers on the sheet or can be used during play.
//!
//! These are flat structs rather than tagged enums so that the UI can edit any field through a
//! JSON Pointer without the value failing to deserialize when its kind changes. Fields that
//! don't apply to a modifier's kind are ignored.

use crate::abilities::Ability;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModifierKind {
    ArmorClass,
    AttackRoll,
    Damage,
    Save,
    Speed,
    Initiative,
    MaxHp,
    Resistance,
    #[default]
    Note,
}

/// Which weapon attacks an attack or damage modifier applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    #[default]
    AllWeapons,
    Melee,
    /// Ranged attacks, including thrown melee weapons.
    Ranged,
    /// Attacks with ranged weapons only (Archery), not thrown melee weapons.
    RangedWeapon,
}

impl Scope {
    pub fn matches(self, ranged_attack: bool, ranged_weapon: bool) -> bool {
        match self {
            Scope::AllWeapons => true,
            Scope::Melee => !ranged_attack,
            Scope::Ranged => ranged_attack,
            Scope::RangedWeapon => ranged_weapon,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Modifier {
    pub kind: ModifierKind,
    /// Flat bonus for armor_class, attack_roll, save, speed, initiative and max_hp.
    pub value: i32,
    /// Keeps `value` following the character, e.g. `{pb}`; empty for a fixed value.
    pub value_formula: String,
    /// For attack_roll and damage.
    pub scope: Scope,
    /// Extra damage for damage modifiers, e.g. `1d6`.
    pub dice: String,
    /// For damage and resistance; resistance accepts `all`.
    pub damage_type: String,
    /// Damage riders that apply to only the first hit each turn (Slash Draw).
    pub once_per_turn: bool,
    /// For save; None means every save.
    pub ability: Option<Ability>,
    /// For note, and an optional label for the others.
    pub text: String,
    /// Can be switched on and off from the Combat tab, like a weapon's Fire mode.
    pub toggleable: bool,
    /// A modifier that's switched off has no effect.
    pub enabled: bool,
}

impl Default for Modifier {
    fn default() -> Self {
        Modifier {
            kind: ModifierKind::Note,
            value: 0,
            value_formula: String::new(),
            scope: Scope::AllWeapons,
            dice: String::new(),
            damage_type: String::new(),
            once_per_turn: false,
            ability: None,
            text: String::new(),
            toggleable: false,
            enabled: true,
        }
    }
}

impl Modifier {
    /// A short human-readable description, e.g. `+2 ranged weapon attacks`.
    pub fn describe(&self) -> String {
        let signed = |n: i32| {
            if n >= 0 {
                format!("+{n}")
            } else {
                n.to_string()
            }
        };
        let scope = match self.scope {
            Scope::AllWeapons => "weapon",
            Scope::Melee => "melee",
            Scope::Ranged => "ranged",
            Scope::RangedWeapon => "ranged weapon",
        };
        let text = match self.kind {
            ModifierKind::ArmorClass => format!("{} AC", signed(self.value)),
            ModifierKind::AttackRoll => format!("{} {scope} attacks", signed(self.value)),
            ModifierKind::Damage => {
                let typ = if self.damage_type.is_empty() {
                    String::new()
                } else {
                    format!(" {}", self.damage_type)
                };
                let once = if self.once_per_turn {
                    ", once per turn"
                } else {
                    ""
                };
                let amount = match (self.dice.trim(), self.value) {
                    ("", n) => signed(n),
                    (dice, 0) => format!("+{dice}"),
                    (dice, n) => format!("+{dice}{}", signed(n)),
                };
                format!("{amount}{typ} damage on {scope} hits{once}")
            }
            ModifierKind::Save => match self.ability {
                Some(a) => format!("{} {} saves", signed(self.value), a.abbrev()),
                None => format!("{} all saves", signed(self.value)),
            },
            ModifierKind::Speed => format!("{} ft speed", signed(self.value)),
            ModifierKind::Initiative => format!("{} initiative", signed(self.value)),
            ModifierKind::MaxHp => format!("{} max HP", signed(self.value)),
            ModifierKind::Resistance => format!("Resistance: {}", self.damage_type),
            ModifierKind::Note => return self.text.clone(),
        };
        if self.text.is_empty() {
            text
        } else {
            format!("{text} ({})", self.text)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activation {
    #[default]
    Action,
    BonusAction,
    Reaction,
    Free,
    Other,
}

/// Something active on the character, from a spell, an action or added by hand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Effect {
    pub name: String,
    pub source: String,
    pub concentration: bool,
    /// Turns left before it ends on its own; 0 lasts until removed.
    pub rounds: u32,
    pub modifiers: Vec<Modifier>,
    pub notes: String,
}

impl Default for Effect {
    fn default() -> Self {
        Effect {
            name: "New effect".into(),
            source: String::new(),
            concentration: false,
            rounds: 0,
            modifiers: Vec::new(),
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cost {
    #[default]
    None,
    /// Uses up the owning item (potions, oil).
    Quantity,
    /// Spends the owning item's charges.
    Charges,
    /// Spends the owning feature's uses.
    Uses,
}

/// What happens with an action's roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollEffect {
    #[default]
    None,
    Heal,
    TempHp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Action {
    pub name: String,
    pub activation: Activation,
    pub description: String,
    pub cost: Cost,
    pub cost_amount: u32,
    /// Dice to roll when used, e.g. `2d4+2`; empty for none.
    pub roll: String,
    /// Keeps `roll` following the character, e.g. `{pb}`; empty for fixed dice.
    pub roll_formula: String,
    pub roll_effect: RollEffect,
    /// The type of damage `roll` deals, if it deals any.
    pub damage_type: String,
    /// Applied to the character when used.
    pub effect: Option<Effect>,
}

impl Default for Action {
    fn default() -> Self {
        Action {
            name: "Use".into(),
            activation: Activation::Action,
            description: String::new(),
            cost: Cost::None,
            cost_amount: 1,
            roll: String::new(),
            roll_formula: String::new(),
            roll_effect: RollEffect::None,
            damage_type: String::new(),
            effect: None,
        }
    }
}

/// What using an action, casting a spell or firing produced, for the caller to roll and report.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Outcome {
    pub label: String,
    /// The item, feature or spell that produced it.
    pub source: String,
    pub roll: String,
    pub roll_effect: RollEffect,
    pub damage_type: String,
    /// A description of the resource spent, e.g. `1 of 3 uses left`.
    pub spent: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_modifiers() {
        let damage = |dice: &str, value| Modifier {
            kind: ModifierKind::Damage,
            dice: dice.into(),
            value,
            ..Default::default()
        };
        assert_eq!(damage("", 1).describe(), "+1 damage on weapon hits");
        assert_eq!(damage("1d6", 0).describe(), "+1d6 damage on weapon hits");
        let rider = Modifier {
            scope: Scope::Melee,
            damage_type: "piercing".into(),
            once_per_turn: true,
            ..damage("1d6", 0)
        };
        assert_eq!(
            rider.describe(),
            "+1d6 piercing damage on melee hits, once per turn"
        );
        let archery = Modifier {
            kind: ModifierKind::AttackRoll,
            scope: Scope::RangedWeapon,
            value: 2,
            ..Default::default()
        };
        assert_eq!(archery.describe(), "+2 ranged weapon attacks");
    }
}
