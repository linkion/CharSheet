//! Playing a character: actions, casting, weapon attacks and hit dice, with their dice
//! rolled and the results applied. Shared by the app and the MCP server, which save the
//! character and log the returned rolls.
//!
//! Each function works on a copy and only changes the character when everything
//! succeeded, so a bad roll expression doesn't spend a use.

use crate::character::{Character, PlayError, WeaponAttack};
use crate::dice::{self, RollMode};
use crate::effects::{Outcome, RollEffect};
use crate::storage::StoredRoll;
use serde::Serialize;

/// What an action did: a short message for a notification and the rolls to log.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Played {
    pub message: String,
    pub rolls: Vec<StoredRoll>,
}

fn transact<T>(
    c: &mut Character,
    f: impl FnOnce(&mut Character) -> Result<T, PlayError>,
) -> Result<T, PlayError> {
    let mut copy = c.clone();
    let value = f(&mut copy)?;
    *c = copy;
    Ok(value)
}

/// Rolls an outcome's dice, applies healing or temp HP, and describes what happened.
fn resolve(c: &mut Character, outcome: Outcome) -> Result<Played, PlayError> {
    let mut parts = vec![outcome.label.clone()];
    let mut rolls = Vec::new();
    if !outcome.roll.trim().is_empty() {
        let rolled = StoredRoll::new(&outcome.label, dice::roll(&outcome.roll, RollMode::Normal)?);
        let total = rolled.result.total;
        parts.push(match outcome.roll_effect {
            RollEffect::None => format!("rolled {total}"),
            RollEffect::Heal => format!("healed {total}"),
            RollEffect::TempHp => format!("{total} temp HP"),
        });
        c.apply_roll(outcome.roll_effect, total);
        rolls.push(rolled);
    }
    if !outcome.spent.is_empty() {
        parts.push(outcome.spent);
    }
    Ok(Played {
        message: parts.join(" · "),
        rolls,
    })
}

/// Takes a long rest, then rolls the charges items regain at dawn.
pub fn long_rest(c: &mut Character) -> Played {
    c.long_rest();
    let mut parts = vec!["Long rest".to_owned()];
    let mut rolls = Vec::new();
    for (name, roll, left) in c.dawn(&mut rand::rng()) {
        parts.push(format!("{name} has {left} charges"));
        rolls.push(StoredRoll::new(format!("{name} regains charges"), roll));
    }
    Played {
        message: parts.join(" · "),
        rolls,
    }
}

pub fn use_item_action(c: &mut Character, item: usize, action: usize) -> Result<Played, PlayError> {
    transact(c, |c| {
        let outcome = c.use_item_action(item, action)?;
        resolve(c, outcome)
    })
}

pub fn use_feature_action(
    c: &mut Character,
    feature: usize,
    action: usize,
) -> Result<Played, PlayError> {
    transact(c, |c| {
        let outcome = c.use_feature_action(feature, action)?;
        resolve(c, outcome)
    })
}

/// Casts a spell with a slot of `slot_level`, or a free use when it's 0.
pub fn cast_spell(c: &mut Character, spell: usize, slot_level: u8) -> Result<Played, PlayError> {
    transact(c, |c| {
        let outcome = c.cast_spell(spell, slot_level)?;
        resolve(c, outcome)
    })
}

/// The attack at `index` in `weapon_attacks()`.
fn nth_attack(c: &Character, index: usize) -> Result<WeaponAttack, PlayError> {
    c.weapon_attacks()
        .into_iter()
        .nth(index)
        .ok_or(PlayError::NotFound("attack"))
}

/// Rolls to hit with a weapon, firing a shot if it's a ranged weapon.
pub fn weapon_attack(
    c: &mut Character,
    attack: usize,
    mode: RollMode,
) -> Result<StoredRoll, PlayError> {
    transact(c, |c| {
        let a = nth_attack(c, attack)?;
        if a.ranged && !a.thrown {
            c.fire(a.item)?;
        }
        let modifier = match a.to_hit {
            0 => String::new(),
            n if n > 0 => format!("+{n}"),
            n => n.to_string(),
        };
        let rolled = dice::roll(&format!("1d20{modifier}"), mode)?;
        Ok(StoredRoll::new(format!("{} attack", a.name), rolled))
    })
}

/// Rolls a weapon's damage with every rider except those named in `excluded`.
pub fn weapon_damage(
    c: &mut Character,
    attack: usize,
    crit: bool,
    excluded: &[String],
) -> Result<StoredRoll, PlayError> {
    transact(c, |c| {
        let a = nth_attack(c, attack)?;
        let expression = c.weapon_damage(&a, crit, excluded)?;
        let label = if crit {
            format!("{} critical damage", a.name)
        } else {
            format!("{} damage", a.name)
        };
        Ok(StoredRoll::new(
            label,
            dice::roll(&expression, RollMode::Normal)?,
        ))
    })
}

/// Spends a hit die of the class at `class_index` and describes the result.
pub fn spend_hit_die(c: &mut Character, class_index: usize) -> Result<String, PlayError> {
    if class_index >= c.classes.len() {
        return Err(PlayError::NotFound("class"));
    }
    match c.spend_hit_die(class_index, &mut rand::rng()) {
        Some((roll, healed)) => Ok(format!("Rolled {roll}, regained {healed} HP")),
        None => Err(PlayError::Unavailable(
            "No hit dice of that type left".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sable() -> Character {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sable.json");
        crate::storage::read_file(&path).expect("examples/sable.json loads")
    }

    fn spell(c: &Character, name: &str) -> usize {
        c.spells.iter().position(|s| s.name == name).unwrap()
    }

    #[test]
    fn weapon_attack_fires_and_rolls() {
        let mut c = sable();
        let i = c
            .weapon_attacks()
            .iter()
            .position(|a| a.name == "Revolver")
            .unwrap();
        let item = c.weapon_attacks()[i].item;
        let roll = weapon_attack(&mut c, i, RollMode::Advantage).unwrap();
        assert_eq!(roll.label, "Revolver attack");
        assert_eq!(roll.result.expression, "1d20+9");
        assert_eq!(roll.result.mode, RollMode::Advantage);
        assert_eq!(c.inventory[item].weapon.as_ref().unwrap().loaded, 5);

        let damage = weapon_damage(&mut c, i, true, &[]).unwrap();
        assert_eq!(damage.label, "Revolver critical damage");
        assert!(matches!(
            weapon_attack(&mut c, 99, RollMode::Normal),
            Err(PlayError::NotFound("attack"))
        ));
    }

    #[test]
    fn healing_action_heals() {
        let mut c = sable();
        let potion = c
            .inventory
            .iter()
            .position(|i| i.name == "Potion of Healing")
            .unwrap();
        let quantity = c.inventory[potion].quantity;
        c.take_damage(20);
        let before = c.hp.current;
        let played = use_item_action(&mut c, potion, 0).unwrap();
        assert_eq!(played.rolls.len(), 1);
        assert_eq!(c.hp.current, before + played.rolls[0].result.total);
        assert_eq!(c.inventory[potion].quantity, quantity - 1);
        assert!(played.message.contains("healed"));
    }

    #[test]
    fn casting_spends_a_slot() {
        let mut c = sable();
        let cure = spell(&c, "Cure Wounds");
        let played = cast_spell(&mut c, cure, 1).unwrap();
        assert_eq!(c.spell_slots_used[0], 1);
        assert_eq!(played.rolls[0].result.expression, "2d8+2");
        assert!(cast_spell(&mut c, cure, 5).is_err());
    }

    #[test]
    fn a_bad_roll_spends_nothing() {
        let mut c = sable();
        let cure = spell(&c, "Cure Wounds");
        c.spells[cure].roll = "2d8+banana".into();
        let before = c.clone();
        assert!(matches!(
            cast_spell(&mut c, cure, 1),
            Err(PlayError::Dice(_))
        ));
        assert_eq!(c, before);
    }

    #[test]
    fn hit_dice() {
        let mut c = sable();
        let levels = c.classes[0].level;
        for _ in 0..levels {
            spend_hit_die(&mut c, 0).unwrap();
        }
        assert!(matches!(
            spend_hit_die(&mut c, 0),
            Err(PlayError::Unavailable(_))
        ));
        assert!(matches!(
            spend_hit_die(&mut c, 9),
            Err(PlayError::NotFound("class"))
        ));
    }

    #[test]
    fn long_rest_logs_dawn_rolls() {
        let mut c = sable();
        c.inventory.push(crate::character::Item {
            name: "Wand".into(),
            charges_max: 7,
            charges_spent: 7,
            recharge: crate::character::Recharge::Dawn,
            regain: "1d6+1".into(),
            ..Default::default()
        });
        let played = long_rest(&mut c);
        assert_eq!(played.rolls.len(), 1);
        assert_eq!(played.rolls[0].label, "Wand regains charges");
        let last = c.inventory.last().unwrap();
        assert_eq!(
            last.charges_max - last.charges_spent,
            played.rolls[0].result.total as u32
        );
        assert!(played.message.contains("Wand has"));
    }
}
