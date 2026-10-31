//! A serializable snapshot of a character plus every derived value, for the UI.

use crate::abilities::{Ability, Proficiency, Skill};
use crate::armor::ArmorKind;
use crate::character::{Character, Condition, ItemKind, Recharge, Toggle, WeaponAttack};
use crate::classes::ClassId;
use crate::effects::{Activation, Cost, Modifier, ModifierKind, RollEffect, Scope};
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

/// A limited resource shown as pips: uses, charges, quantity or free casts.
#[derive(Serialize)]
pub struct ResourceView {
    pub label: String,
    pub left: u32,
    pub max: u32,
}

/// Something the character can do from the Combat tab: an item or feature action, or a spell
/// that rolls or applies an effect.
#[derive(Serialize)]
pub struct ActionView {
    /// "item", "feature" or "spell".
    pub owner: &'static str,
    pub index: usize,
    /// Index of the action on its owner; 0 for spells.
    pub action: usize,
    pub name: String,
    pub source: String,
    pub activation: Activation,
    pub description: String,
    pub roll: String,
    pub resource: Option<ResourceView>,
    pub available: bool,
    pub concentration: bool,
    /// Lowest slot level for spells, 0 for cantrips and non-spells.
    pub spell_level: u8,
}

#[derive(Serialize)]
pub struct NoteView {
    pub source: String,
    pub text: String,
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
    pub max_hp: i32,
    pub resistances: Vec<String>,
    pub weapon_attacks: Vec<WeaponAttack>,
    /// Modifiers that can be switched on and off.
    pub toggles: Vec<Toggle>,
    pub notes_in_force: Vec<NoteView>,
    /// Name of the effect being concentrated on, if any.
    pub concentrating: Option<String>,
    /// One-line summaries of each active effect's modifiers, in `character.effects` order.
    pub effect_summaries: Vec<String>,
    pub item_summaries: Vec<String>,
    pub feature_summaries: Vec<String>,
    pub actions: Vec<ActionView>,
}

fn summarize(modifiers: &[Modifier]) -> String {
    modifiers
        .iter()
        .map(|m| match m.describe() {
            s if s.is_empty() || m.enabled => s,
            s => format!("{s} (off)"),
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

fn actions(c: &Character) -> Vec<ActionView> {
    let mut out = Vec::new();
    for (index, item) in c.inventory.iter().enumerate() {
        for (i, a) in item.actions.iter().enumerate() {
            let resource = match a.cost {
                Cost::None => None,
                Cost::Quantity => Some(ResourceView {
                    label: "left".into(),
                    left: item.quantity,
                    max: item.quantity,
                }),
                Cost::Charges | Cost::Uses => Some(ResourceView {
                    label: "charges".into(),
                    left: item.charges_max.saturating_sub(item.charges_spent),
                    max: item.charges_max,
                }),
            };
            out.push(ActionView {
                owner: "item",
                index,
                action: i,
                name: a.name.clone(),
                source: item.name.clone(),
                activation: a.activation,
                description: a.description.clone(),
                roll: a.roll.clone(),
                available: resource.as_ref().is_none_or(|r| r.left >= a.cost_amount),
                resource,
                concentration: a.effect.as_ref().is_some_and(|e| e.concentration),
                spell_level: 0,
            });
        }
    }
    for (index, f) in c.features.iter().enumerate() {
        for (i, a) in f.actions.iter().enumerate() {
            let resource = (a.cost != Cost::None).then(|| ResourceView {
                label: "uses".into(),
                left: f.uses_max.saturating_sub(f.uses_spent),
                max: f.uses_max,
            });
            out.push(ActionView {
                owner: "feature",
                index,
                action: i,
                name: a.name.clone(),
                source: f.name.clone(),
                activation: a.activation,
                description: a.description.clone(),
                roll: a.roll.clone(),
                available: resource.as_ref().is_none_or(|r| r.left >= a.cost_amount),
                resource,
                concentration: a.effect.as_ref().is_some_and(|e| e.concentration),
                spell_level: 0,
            });
        }
    }
    let slots = c.spell_slots_max();
    let (pact_max, pact_level) = c.pact_slots();
    for (index, s) in c.spells.iter().enumerate() {
        if !s.prepared || (s.effect.is_none() && s.roll.trim().is_empty()) {
            continue;
        }
        let free_left = s.free_uses_max.saturating_sub(s.free_uses_spent);
        let slot_left = (s.level.max(1)..=9).any(|l| {
            let i = l as usize - 1;
            c.spell_slots_used[i] < slots[i] || (l == pact_level && c.pact_slots_used < pact_max)
        });
        out.push(ActionView {
            owner: "spell",
            index,
            action: 0,
            name: s.name.clone(),
            source: if s.level == 0 {
                "Cantrip".into()
            } else {
                format!("Level {} spell", s.level)
            },
            activation: s.casting_time,
            description: s.notes.clone(),
            roll: s.roll.clone(),
            resource: (s.free_uses_max > 0).then(|| ResourceView {
                label: "free casts".into(),
                left: free_left,
                max: s.free_uses_max,
            }),
            available: s.level == 0 || free_left > 0 || slot_left,
            concentration: s.concentration || s.effect.as_ref().is_some_and(|e| e.concentration),
            spell_level: s.level,
        });
    }
    out
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
    pub item_kinds: Vec<Choice>,
    pub modifier_kinds: Vec<Choice>,
    pub scopes: Vec<Choice>,
    pub activations: Vec<Choice>,
    pub costs: Vec<Choice>,
    pub roll_effects: Vec<Choice>,
    pub recharges: Vec<Choice>,
}

fn choices<T: Serialize + Copy>(list: &[(T, &'static str)]) -> Vec<Choice> {
    list.iter()
        .map(|&(k, name)| Choice { key: key(k), name })
        .collect()
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
            max_hp: c.max_hp(),
            resistances: c.resistances(),
            weapon_attacks: c.weapon_attacks(),
            toggles: c.toggles(),
            notes_in_force: c
                .notes_in_force()
                .into_iter()
                .map(|(source, text)| NoteView { source, text })
                .collect(),
            concentrating: c.concentrating_on().map(|e| e.name.clone()),
            effect_summaries: c.effects.iter().map(|e| summarize(&e.modifiers)).collect(),
            item_summaries: c
                .inventory
                .iter()
                .map(|i| summarize(&i.modifiers))
                .collect(),
            feature_summaries: c.features.iter().map(|f| summarize(&f.modifiers)).collect(),
            actions: actions(c),
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
        item_kinds: choices(&[
            (ItemKind::Gear, "Gear"),
            (ItemKind::Weapon, "Weapon"),
            (ItemKind::Armor, "Armor"),
            (ItemKind::Shield, "Shield"),
            (ItemKind::Ammunition, "Ammunition"),
            (ItemKind::Consumable, "Consumable"),
        ]),
        modifier_kinds: choices(&[
            (ModifierKind::ArmorClass, "Armor Class"),
            (ModifierKind::AttackRoll, "Attack rolls"),
            (ModifierKind::Damage, "Damage"),
            (ModifierKind::Save, "Saving throws"),
            (ModifierKind::Speed, "Speed"),
            (ModifierKind::Initiative, "Initiative"),
            (ModifierKind::MaxHp, "Hit point maximum"),
            (ModifierKind::Resistance, "Resistance"),
            (ModifierKind::Note, "Note"),
        ]),
        scopes: choices(&[
            (Scope::AllWeapons, "All weapon attacks"),
            (Scope::Melee, "Melee attacks"),
            (Scope::Ranged, "Ranged attacks (incl. thrown)"),
            (Scope::RangedWeapon, "Ranged weapons only"),
        ]),
        activations: choices(&[
            (Activation::Action, "Action"),
            (Activation::BonusAction, "Bonus Action"),
            (Activation::Reaction, "Reaction"),
            (Activation::Free, "Free"),
            (Activation::Other, "Other"),
        ]),
        costs: choices(&[
            (Cost::None, "Nothing"),
            (Cost::Quantity, "Uses up the item"),
            (Cost::Charges, "Item charges"),
            (Cost::Uses, "Feature uses"),
        ]),
        roll_effects: choices(&[
            (RollEffect::None, "Just roll"),
            (RollEffect::Heal, "Heal by the total"),
            (RollEffect::TempHp, "Gain temp HP"),
        ]),
        recharges: choices(&[
            (Recharge::None, "Doesn't recharge"),
            (Recharge::Short, "Short or Long Rest"),
            (Recharge::Long, "Long Rest"),
            (Recharge::Dawn, "Next dawn"),
        ]),
    }
}
