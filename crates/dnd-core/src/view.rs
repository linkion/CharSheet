//! A serializable snapshot of a character plus every derived value, for the UI.

use crate::abilities::{Ability, Proficiency, Skill};
use crate::armor::ArmorKind;
use crate::character::{Character, Condition};
use crate::classes::ClassId;
use serde::Serialize;

#[derive(Serialize)]
pub struct AbilityView {
    pub key: Ability,
    pub name: &'static str,
    pub abbrev: &'static str,
    pub score: i32,
    pub modifier: i32,
    pub save: i32,
    pub save_proficient: bool,
    pub save_from_class: bool,
}

#[derive(Serialize)]
pub struct SkillView {
    pub key: Skill,
    pub name: &'static str,
    pub ability: &'static str,
    pub modifier: i32,
    pub proficiency: Proficiency,
}

#[derive(Serialize)]
pub struct CastingView {
    pub class: String,
    pub ability: &'static str,
    pub save_dc: i32,
    pub attack: i32,
}

#[derive(Serialize)]
pub struct AttackView {
    pub to_hit: i32,
    pub damage: String,
}

#[derive(Serialize)]
pub struct HitDiceView {
    pub class: &'static str,
    pub die: u8,
    pub total: u32,
    pub remaining: u32,
}

#[derive(Serialize)]
pub struct Derived {
    pub total_level: u32,
    pub class_summary: String,
    pub proficiency_bonus: i32,
    pub abilities: Vec<AbilityView>,
    pub skills: Vec<SkillView>,
    pub armor_class: i32,
    pub initiative: i32,
    pub speed: i32,
    pub passive_perception: i32,
    pub passive_insight: i32,
    pub passive_investigation: i32,
    pub d20_penalty: i32,
    pub casting: Vec<CastingView>,
    pub spell_slots: [u8; 9],
    pub pact_slots: u8,
    pub pact_slot_level: u8,
    pub attacks: Vec<AttackView>,
    pub hit_dice: Vec<HitDiceView>,
    pub carrying_capacity: f64,
    pub carried_weight: f64,
    pub attuned: usize,
}

#[derive(Serialize)]
pub struct Choice {
    pub key: serde_json::Value,
    pub name: &'static str,
}

/// Static choice lists for the UI's combo boxes.
#[derive(Serialize)]
pub struct Catalog {
    pub classes: Vec<Choice>,
    pub armor: Vec<Choice>,
    pub abilities: Vec<Choice>,
    pub conditions: Vec<Choice>,
}

#[derive(Serialize)]
pub struct SheetView<'a> {
    pub character: &'a Character,
    pub derived: Derived,
}

impl<'a> SheetView<'a> {
    pub fn new(c: &'a Character) -> SheetView<'a> {
        let first_class_saves = c
            .classes
            .first()
            .map(|cl| cl.class.info().saves)
            .unwrap_or([Ability::Str; 2]);
        let has_class = !c.classes.is_empty();
        let derived = Derived {
            total_level: c.total_level(),
            class_summary: c.class_summary(),
            proficiency_bonus: c.proficiency_bonus(),
            abilities: Ability::ALL
                .iter()
                .map(|&a| AbilityView {
                    key: a,
                    name: a.name(),
                    abbrev: a.abbrev(),
                    score: c.score(a),
                    modifier: c.ability_mod(a),
                    save: c.save_mod(a),
                    save_proficient: c.save_proficient(a),
                    save_from_class: has_class && first_class_saves.contains(&a),
                })
                .collect(),
            skills: Skill::ALL
                .iter()
                .map(|&s| SkillView {
                    key: s,
                    name: s.name(),
                    ability: s.ability().abbrev(),
                    modifier: c.skill_mod(s),
                    proficiency: c.skill_proficiency(s),
                })
                .collect(),
            armor_class: c.armor_class(),
            initiative: c.initiative(),
            speed: c.speed(),
            passive_perception: c.passive(Skill::Perception),
            passive_insight: c.passive(Skill::Insight),
            passive_investigation: c.passive(Skill::Investigation),
            d20_penalty: c.d20_penalty(),
            casting: c
                .spellcasting_abilities()
                .into_iter()
                .map(|(class, a)| CastingView {
                    class,
                    ability: a.abbrev(),
                    save_dc: c.spell_save_dc(a),
                    attack: c.spell_attack(a),
                })
                .collect(),
            spell_slots: c.spell_slots_max(),
            pact_slots: c.pact_slots().0,
            pact_slot_level: c.pact_slots().1,
            attacks: c
                .attacks
                .iter()
                .map(|a| AttackView {
                    to_hit: c.attack_bonus(a),
                    damage: c.damage_expression(a),
                })
                .collect(),
            hit_dice: c
                .classes
                .iter()
                .map(|cl| {
                    let info = cl.class.info();
                    HitDiceView {
                        class: info.name,
                        die: info.hit_die,
                        total: cl.level,
                        remaining: cl.level - cl.hit_dice_spent.min(cl.level),
                    }
                })
                .collect(),
            carrying_capacity: c.carrying_capacity(),
            carried_weight: c.carried_weight(),
            attuned: c.attuned_count(),
        };
        SheetView {
            character: c,
            derived,
        }
    }
}

fn key<T: Serialize>(value: T) -> serde_json::Value {
    serde_json::to_value(value).expect("enum keys serialize")
}

pub fn catalog() -> Catalog {
    Catalog {
        classes: ClassId::ALL
            .iter()
            .map(|&c| Choice {
                key: key(c),
                name: c.info().name,
            })
            .collect(),
        armor: ArmorKind::ALL
            .iter()
            .map(|&a| Choice {
                key: key(a),
                name: a.info().name,
            })
            .collect(),
        abilities: Ability::ALL
            .iter()
            .map(|&a| Choice {
                key: key(a),
                name: a.name(),
            })
            .collect(),
        conditions: Condition::ALL
            .iter()
            .map(|&c| {
                let k = key(c);
                let name = match c {
                    Condition::Blinded => "Blinded",
                    Condition::Charmed => "Charmed",
                    Condition::Deafened => "Deafened",
                    Condition::Frightened => "Frightened",
                    Condition::Grappled => "Grappled",
                    Condition::Incapacitated => "Incapacitated",
                    Condition::Invisible => "Invisible",
                    Condition::Paralyzed => "Paralyzed",
                    Condition::Petrified => "Petrified",
                    Condition::Poisoned => "Poisoned",
                    Condition::Prone => "Prone",
                    Condition::Restrained => "Restrained",
                    Condition::Stunned => "Stunned",
                    Condition::Unconscious => "Unconscious",
                };
                Choice { key: k, name }
            })
            .collect(),
    }
}
