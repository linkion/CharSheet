//! Playing a character: actions, casting, weapon attacks and hit dice, with their dice
//! rolled and the results applied. Shared by the app and the MCP server, which save the
//! character and log the returned rolls.
//!
//! Each function works on a copy and only changes the character when everything
//! succeeded, so a bad roll expression doesn't spend a use.

use crate::abilities::{Ability, Skill};
use crate::character::{Character, PlayError, WeaponAttack};
use crate::dice::{self, Bonus, RollMode, RollPart};
use crate::effects::{Outcome, RollEffect};
use crate::storage::StoredRoll;
use serde::{Deserialize, Serialize};

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
        let part = RollPart::new(&outcome.roll, &outcome.source).typed(&outcome.damage_type);
        let rolled = StoredRoll::new(&outcome.label, dice::roll_parts(&[part], RollMode::Normal)?);
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
        let rolled = roll_d20(&a.to_hit_parts, mode)?;
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
        let parts = c.weapon_damage(&a, crit, excluded)?;
        Ok(StoredRoll::new(
            damage_label(&a.name, crit),
            dice::roll_parts(&parts, RollMode::Normal)?,
        ))
    })
}

fn damage_label(name: &str, crit: bool) -> String {
    if crit {
        format!("{name} critical damage")
    } else {
        format!("{name} damage")
    }
}

/// A d20 test with `bonuses` added.
fn roll_d20(bonuses: &[Bonus], mode: RollMode) -> Result<dice::RollResult, PlayError> {
    let mut parts = vec![RollPart::new("1d20", "d20")];
    parts.extend(bonuses.iter().map(Bonus::part));
    Ok(dice::roll_parts(&parts, mode)?)
}

/// A roll made from the sheet that needs no resources, named by what it's for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RollTarget {
    Ability { ability: Ability },
    Save { ability: Ability },
    Skill { skill: Skill },
    Initiative,
    SpellAttack { ability: Ability },
    ManualAttack { attack: usize },
    ManualDamage { attack: usize, crit: bool },
    DeathSave,
    Concentration { dc: i32 },
}

/// Makes a roll from the sheet. `mode` applies to d20 tests, not damage.
pub fn roll_check(
    c: &Character,
    target: RollTarget,
    mode: RollMode,
) -> Result<StoredRoll, PlayError> {
    let d20 =
        |label: String, bonuses: Vec<Bonus>| Ok(StoredRoll::new(label, roll_d20(&bonuses, mode)?));
    let manual = |attack: usize| c.attacks.get(attack).ok_or(PlayError::NotFound("attack"));
    match target {
        RollTarget::Ability { ability } => {
            d20(format!("{} check", ability.name()), c.check_parts(ability))
        }
        RollTarget::Save { ability } => {
            d20(format!("{} save", ability.name()), c.save_parts(ability))
        }
        RollTarget::Skill { skill } => d20(skill.name().to_owned(), c.skill_parts(skill)),
        RollTarget::Initiative => d20("Initiative".to_owned(), c.initiative_parts()),
        RollTarget::SpellAttack { ability } => {
            d20("Spell attack".to_owned(), c.spell_attack_parts(ability))
        }
        RollTarget::ManualAttack { attack } => {
            let a = manual(attack)?;
            d20(format!("{} attack", a.name), c.attack_parts(a))
        }
        RollTarget::ManualDamage { attack, crit } => {
            let a = manual(attack)?;
            let mut parts = c.attack_damage_parts(a);
            if crit {
                parts = parts
                    .into_iter()
                    .map(RollPart::doubled)
                    .collect::<Result<_, _>>()?;
            }
            Ok(StoredRoll::new(
                damage_label(&a.name, crit),
                dice::roll_parts(&parts, RollMode::Normal)?,
            ))
        }
        RollTarget::DeathSave => d20(
            "Death save".to_owned(),
            vec![Bonus::new(c.d20_penalty(), "Exhaustion")],
        ),
        RollTarget::Concentration { dc } => d20(
            format!("Concentration save (DC {dc})"),
            c.save_parts(Ability::Con),
        ),
    }
}

/// Spends a hit die of the class at `class_index`, logging its roll.
pub fn spend_hit_die(c: &mut Character, class_index: usize) -> Result<Played, PlayError> {
    if class_index >= c.classes.len() {
        return Err(PlayError::NotFound("class"));
    }
    match c.spend_hit_die(class_index, &mut rand::rng()) {
        Some((roll, healed)) => Ok(Played {
            message: format!("Rolled {}, regained {healed} HP", roll.terms[0].value),
            rolls: vec![StoredRoll::new("Hit die", roll)],
        }),
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

    fn term<'a>(roll: &'a StoredRoll, source: &str) -> &'a dice::TermResult {
        roll.result
            .terms
            .iter()
            .find(|t| t.source == source)
            .unwrap_or_else(|| panic!("no {source} in {:?}", roll.result.terms))
    }

    fn sum(roll: &StoredRoll) -> i32 {
        roll.result.terms.iter().map(|t| t.sign * t.value).sum()
    }

    #[test]
    fn checks_name_their_sources_and_add_up() {
        let mut c = sable();
        c.exhaustion = 1;
        let roll = roll_check(
            &c,
            RollTarget::Save {
                ability: Ability::Dex,
            },
            RollMode::Advantage,
        )
        .unwrap();
        assert_eq!(roll.label, "Dexterity save");
        assert_eq!(roll.result.mode, RollMode::Advantage);
        assert_eq!(term(&roll, "d20").label, "2d20kh1");
        assert_eq!(
            term(&roll, "Exhaustion").sign * term(&roll, "Exhaustion").value,
            -2
        );
        assert_eq!(sum(&roll), roll.result.total);
        let d20 = term(&roll, "d20").value;
        assert_eq!(roll.result.total, d20 + c.save_mod(Ability::Dex));
    }

    #[test]
    fn exhaustion_applies_to_ability_checks_but_not_passives() {
        let mut c = sable();
        let fresh = Bonus::total(&c.check_parts(Ability::Wis));
        let passive = c.passive(Skill::Perception);
        c.exhaustion = 2;
        assert_eq!(Bonus::total(&c.check_parts(Ability::Wis)), fresh - 4);
        assert_eq!(c.passive(Skill::Perception), passive);
        let roll = roll_check(
            &c,
            RollTarget::Ability {
                ability: Ability::Wis,
            },
            RollMode::Normal,
        )
        .unwrap();
        assert_eq!(roll.label, "Wisdom check");
        assert_eq!(term(&roll, "Exhaustion").value, 4);
    }

    #[test]
    fn weapon_damage_splits_by_type_and_source() {
        let mut c = sable();
        let i = c
            .weapon_attacks()
            .iter()
            .position(|a| !a.riders.is_empty())
            .unwrap();
        let attack = c.weapon_attacks()[i].clone();
        let rider = attack.riders[0].clone();
        let roll = weapon_damage(&mut c, i, false, &[]).unwrap();
        assert_eq!(roll.result.total, sum(&roll));
        assert_eq!(term(&roll, &rider.name).source, rider.name);
        let by_type = roll.result.damage_by_type();
        assert_eq!(
            by_type.iter().map(|(_, n)| n).sum::<i32>(),
            roll.result.total
        );
        assert_eq!(by_type[0].0, attack.damage_type);
    }

    #[test]
    fn manual_damage_is_typed_and_doubles_on_a_crit() {
        let mut c = sable();
        c.attacks = vec![crate::character::Attack {
            name: "Claw".into(),
            damage: "1d6".into(),
            damage_type: "slashing".into(),
            bonus: 1,
            add_ability_to_damage: false,
            ..Default::default()
        }];
        let target = RollTarget::ManualDamage {
            attack: 0,
            crit: true,
        };
        let roll = roll_check(&c, target, RollMode::Advantage).unwrap();
        assert_eq!(roll.label, "Claw critical damage");
        assert_eq!(roll.result.expression, "2d6+1");
        assert_eq!(roll.result.mode, RollMode::Normal);
        assert!(
            roll.result
                .terms
                .iter()
                .all(|t| t.damage_type == "slashing")
        );
        let missing = RollTarget::ManualAttack { attack: 5 };
        assert!(roll_check(&c, missing, RollMode::Normal).is_err());
    }

    #[test]
    fn spell_rolls_carry_source_and_type() {
        let mut c = sable();
        let cure = spell(&c, "Cure Wounds");
        c.spells[cure].damage_type = "radiant".into();
        let played = cast_spell(&mut c, cure, 1).unwrap();
        let roll = &played.rolls[0];
        assert!(roll.result.terms.iter().all(|t| t.source == "Cure Wounds"));
        assert!(roll.result.terms.iter().all(|t| t.damage_type == "radiant"));
    }

    #[test]
    fn roll_targets_read_from_json() {
        let target: RollTarget =
            serde_json::from_str(r#"{"kind":"skill","skill":"animal_handling"}"#).unwrap();
        assert_eq!(
            target,
            RollTarget::Skill {
                skill: Skill::AnimalHandling
            }
        );
        let target: RollTarget =
            serde_json::from_str(r#"{"kind":"manual_damage","attack":1,"crit":false}"#).unwrap();
        assert_eq!(
            target,
            RollTarget::ManualDamage {
                attack: 1,
                crit: false
            }
        );
    }

    #[test]
    fn hit_dice_are_logged_with_their_parts() {
        let mut c = sable();
        let played = spend_hit_die(&mut c, 0).unwrap();
        let roll = &played.rolls[0];
        assert_eq!(roll.label, "Hit die");
        assert_eq!(roll.result.terms[0].source, c.classes[0].class.info().name);
        assert_eq!(sum(roll), roll.result.total);
        assert_eq!(
            roll.result.total,
            roll.result.terms[0].value + c.ability_mod(Ability::Con)
        );
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
