//! Turning SRD records into sheet entries. Only structured fields are read here: prose is
//! copied across for the player, never interpreted.

use super::data::{record, records};
use super::mechanics::{self, Choice, Mechanics};
use crate::armor::ArmorKind;
use crate::character::{Character, Feature, Item, ItemKind, Recharge, Spell, WeaponStats};
use crate::edit::{self, EditError};
use crate::effects::{Action, Activation, Cost, Effect, Modifier, ModifierKind};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
pub enum SrdError {
    #[error("no SRD record {0}")]
    NotFound(String),
    #[error("{0} can't be added to a character; {1}")]
    NotAddable(String, &'static str),
    #[error("{name} needs a {what}: one of {}", .allowed.join(", "))]
    NeedsOption {
        name: String,
        what: &'static str,
        allowed: Vec<String>,
    },
    #[error(transparent)]
    Edit(#[from] EditError),
    #[error("{0} is already on the sheet")]
    Duplicate(String),
}

/// What to add, and the choices a record leaves open.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct AddSpec {
    pub id: String,
    /// The mundane weapon or armor a magic item is made from, e.g. `weapon:longsword`.
    pub base: Option<String>,
    /// e.g. `+1`, or `Arrows` for ammunition.
    pub variant: Option<String>,
    /// A choice the record leaves open, e.g. a damage type.
    pub choice: Option<String>,
    pub quantity: Option<u32>,
    pub equipped: Option<bool>,
    pub attuned: Option<bool>,
    pub prepared: Option<bool>,
}

/// A sheet entry made from a record.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Entry {
    Item(Box<Item>),
    Spell(Box<Spell>),
    Feature(Box<Feature>),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Conversion {
    pub entry: Entry,
    /// What was filled in beyond the text, for review.
    pub derived: Vec<String>,
    /// What the entry won't do until the player acts, or may get wrong.
    pub warnings: Vec<String>,
}

/// What a record leaves open, for a picker to offer.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Options {
    /// Ids of the weapons or armor a magic item can be, or names of ammunition.
    pub bases: Vec<Base>,
    pub variants: Vec<String>,
    pub choice: Option<Choice>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Base {
    pub id: String,
    pub name: String,
}

/// Gear used up as it's used.
const CONSUMABLE_GEAR: [&str; 7] = [
    "gear:acid",
    "gear:alchemists-fire",
    "gear:antitoxin",
    "gear:holy-water",
    "gear:oil",
    "gear:poison-basic",
    "gear:rations",
];

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

fn weight(value: &Value) -> f64 {
    value.as_f64().unwrap_or(0.0)
}

fn name_of(rec: &Value) -> String {
    text(&rec["name"])
}

fn found(id: &str) -> Result<Value, SrdError> {
    record(id).ok_or_else(|| SrdError::NotFound(id.to_owned()))
}

/// The printed form of a weapon property, e.g. `Versatile (1d10)`.
fn property_label(p: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(n) = p["range"]["normal"].as_u64() {
        parts.push(format!("{n}/{}", p["range"]["long"].as_u64().unwrap_or(n)));
    }
    for key in ["ammunition", "damage", "note"] {
        if let Some(value) = p[key].as_str() {
            parts.push(value.to_owned());
        }
    }
    match parts.as_slice() {
        [] => name_of(p),
        parts => format!("{} ({})", name_of(p), parts.join("; ")),
    }
}

pub(super) fn weapon_stats(rec: &Value) -> WeaponStats {
    let properties = rec["properties"].as_array().map_or(&[][..], Vec::as_slice);
    let has = |id: &str| properties.iter().any(|p| p["id"] == id);
    let mastery = rec["mastery"]
        .as_str()
        .and_then(record)
        .map(|m| name_of(&m))
        .unwrap_or_default();
    let range = properties.iter().find_map(|p| {
        let normal = p["range"]["normal"].as_u64()?;
        Some(format!("{normal}/{}", p["range"]["long"].as_u64()?))
    });
    WeaponStats {
        damage: text(&rec["damage"]["dice"]),
        damage_type: text(&rec["damage"]["type"]),
        finesse: has("weapon_property:finesse"),
        ranged: rec["ranged"].as_bool().unwrap_or(false),
        thrown: has("weapon_property:thrown"),
        light: has("weapon_property:light"),
        range: range.unwrap_or_default(),
        mastery,
        versatile: properties
            .iter()
            .find_map(|p| p["damage"].as_str())
            .unwrap_or_default()
            .to_owned(),
        properties: properties.iter().map(property_label).collect(),
        ..Default::default()
    }
}

/// The app's armor for a record, which is `None` for a shield.
pub(super) fn armor_kind(id: &str) -> Option<ArmorKind> {
    Some(match id {
        "armor:padded-armor" => ArmorKind::Padded,
        "armor:leather-armor" => ArmorKind::Leather,
        "armor:studded-leather-armor" => ArmorKind::StuddedLeather,
        "armor:hide-armor" => ArmorKind::Hide,
        "armor:chain-shirt" => ArmorKind::ChainShirt,
        "armor:scale-mail" => ArmorKind::ScaleMail,
        "armor:breastplate" => ArmorKind::Breastplate,
        "armor:half-plate-armor" => ArmorKind::HalfPlate,
        "armor:ring-mail" => ArmorKind::RingMail,
        "armor:chain-mail" => ArmorKind::ChainMail,
        "armor:splint-armor" => ArmorKind::Splint,
        "armor:plate-armor" => ArmorKind::Plate,
        _ => return None,
    })
}

fn weapon_item(rec: &Value) -> Item {
    Item {
        name: name_of(rec),
        weight: weight(&rec["weight_lb"]),
        kind: ItemKind::Weapon,
        weapon: Some(weapon_stats(rec)),
        ..Default::default()
    }
}

fn armor_item(rec: &Value) -> Item {
    let id = text(&rec["id"]);
    Item {
        name: name_of(rec),
        weight: weight(&rec["weight_lb"]),
        notes: text(&rec["don_doff"]),
        kind: match id.as_str() {
            "armor:shield" => ItemKind::Shield,
            _ => ItemKind::Armor,
        },
        armor: armor_kind(&id),
        ..Default::default()
    }
}

fn gear_item(rec: &Value, spec: &AddSpec) -> Result<Item, SrdError> {
    let id = text(&rec["id"]);
    let variants = rec["variants"].as_array().map_or(&[][..], Vec::as_slice);
    let allowed = || variants.iter().map(name_of).collect::<Vec<_>>();
    let variant = match &spec.variant {
        Some(wanted) => Some(
            variants
                .iter()
                .find(|v| v["name"] == wanted.as_str())
                .ok_or_else(|| SrdError::NeedsOption {
                    name: name_of(rec),
                    what: "variant",
                    allowed: allowed(),
                })?,
        ),
        None => None,
    };
    if id == "gear:ammunition" {
        let variant = variant.ok_or_else(|| SrdError::NeedsOption {
            name: name_of(rec),
            what: "variant",
            allowed: allowed(),
        })?;
        let amount = text(&variant["amount"]).parse().unwrap_or(1u32);
        return Ok(Item {
            name: name_of(variant),
            quantity: amount,
            weight: weight(&variant["weight_lb"]) / amount.max(1) as f64,
            notes: text(&rec["markdown"]),
            kind: ItemKind::Ammunition,
            ..Default::default()
        });
    }
    Ok(Item {
        name: variant.map_or_else(|| name_of(rec), name_of),
        weight: weight(&variant.unwrap_or(rec)["weight_lb"]),
        notes: text(&rec["markdown"]),
        kind: if CONSUMABLE_GEAR.contains(&id.as_str()) {
            ItemKind::Consumable
        } else {
            ItemKind::Gear
        },
        ..Default::default()
    })
}

fn tool_item(rec: &Value) -> Item {
    Item {
        name: name_of(rec),
        weight: weight(&rec["weight_lb"]),
        notes: text(&rec["markdown"]),
        ..Default::default()
    }
}

/// Replaces `{choice}` in the strings of a modifier.
fn chosen_modifiers(list: &[Modifier], choice: Option<&str>) -> Vec<Modifier> {
    let fill = |text: &str| text.replace("{choice}", choice.unwrap_or_default());
    list.iter()
        .map(|m| Modifier {
            damage_type: fill(&m.damage_type),
            text: fill(&m.text),
            ..m.clone()
        })
        .collect()
}

fn chosen_actions(list: &[Action], choice: Option<&str>) -> Vec<Action> {
    list.iter()
        .map(|a| Action {
            effect: a.effect.as_ref().map(|e| Effect {
                modifiers: chosen_modifiers(&e.modifiers, choice),
                ..e.clone()
            }),
            ..a.clone()
        })
        .collect()
}

/// The names in a `subtype` such as `Glaive, Longsword, or Scimitar`.
fn listed_names(subtype: &str) -> Vec<&str> {
    subtype
        .split(", ")
        .flat_map(|part| part.split(" or "))
        .map(|name| name.trim().trim_start_matches("or ").trim())
        .filter(|name| !name.is_empty())
        .collect()
}

fn base_of(rec: &Value) -> Base {
    Base {
        id: text(&rec["id"]),
        name: name_of(rec),
    }
}

fn bases(rec: &Value, m: Option<&Mechanics>) -> Vec<Base> {
    if let Some(weapon) = m.map(|m| &m.as_weapon).filter(|w| !w.is_empty()) {
        return record(weapon).iter().map(base_of).collect();
    }
    let kind = rec["item_type"].as_str().unwrap_or_default();
    let Some(subtype) = rec["subtype"].as_str() else {
        return Vec::new();
    };
    let armor_tier = |rec: &&Value| rec["armor_category"] != "shield";
    match (kind, subtype) {
        ("weapon", "Any Ammunition") => records("gear.json")
            .iter()
            .filter(|r| r["id"] == "gear:ammunition")
            .flat_map(|r| r["variants"].as_array().into_iter().flatten())
            .map(|v| Base {
                id: name_of(v),
                name: name_of(v),
            })
            .collect(),
        ("weapon", "Any Simple or Martial") => {
            records("weapons.json").iter().map(base_of).collect()
        }
        ("weapon", "Any Melee Weapon") => records("weapons.json")
            .iter()
            .filter(|r| r["ranged"] == false)
            .map(base_of)
            .collect(),
        ("armor", "Any Light, Medium, or Heavy") => records("armor.json")
            .iter()
            .filter(armor_tier)
            .map(base_of)
            .collect(),
        ("armor", "Any Medium or Heavy, Except Hide Armor") => records("armor.json")
            .iter()
            .filter(armor_tier)
            .filter(|r| r["armor_category"] != "light" && r["id"] != "armor:hide-armor")
            .map(base_of)
            .collect(),
        (kind, subtype) => {
            let file = if kind == "armor" {
                "armor.json"
            } else {
                "weapons.json"
            };
            let names = listed_names(subtype);
            records(file)
                .iter()
                .filter(|r| names.iter().any(|n| r["name"] == *n))
                .map(base_of)
                .collect()
        }
    }
}

/// What has to be decided before a record can be added.
pub fn options(id: &str) -> Result<Options, SrdError> {
    let rec = found(id)?;
    let m = mechanics::of(id);
    let mut variants: Vec<String> = rec["rarity_by_variant"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|v| text(&v["variant"]))
        .collect();
    if let Some(m) = m {
        variants.extend(m.tiers.iter().map(|t| t.variant.clone()));
    }
    if id == "gear:ammunition" || rec["variants"].is_array() {
        variants.extend(
            rec["variants"]
                .as_array()
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .map(name_of),
        );
    }
    Ok(Options {
        bases: bases(&rec, m),
        variants,
        choice: m.and_then(|m| m.choice.clone()),
    })
}

/// A chosen option, which has to be allowed and is required when there's more than one.
fn pick<'a>(
    name: &str,
    what: &'static str,
    wanted: Option<&'a str>,
    allowed: &'a [String],
) -> Result<Option<&'a str>, SrdError> {
    if allowed.is_empty() {
        return Ok(None);
    }
    let fail = || SrdError::NeedsOption {
        name: name.to_owned(),
        what,
        allowed: allowed.to_vec(),
    };
    match (wanted, allowed) {
        (Some(wanted), _) => allowed
            .iter()
            .find(|a| a.eq_ignore_ascii_case(wanted))
            .map(|a| Some(a.as_str()))
            .ok_or_else(fail),
        (None, [only]) => Ok(Some(only)),
        (None, _) => Err(fail()),
    }
}

fn magic_item(
    rec: &Value,
    spec: &AddSpec,
    m: Option<&Mechanics>,
    derived: &mut Vec<String>,
) -> Result<Item, SrdError> {
    let id = text(&rec["id"]);
    let name = name_of(rec);
    let open = options(&id)?;
    let base_ids: Vec<String> = open.bases.iter().map(|b| b.id.clone()).collect();
    let base = pick(&name, "base", spec.base.as_deref(), &base_ids)?;
    let variant = pick(&name, "variant", spec.variant.as_deref(), &open.variants)?;
    let allowed = open.choice.as_ref().map_or(&[][..], |c| &c.allowed[..]);
    let choice = pick(&name, "choice", spec.choice.as_deref(), allowed)?;
    let tier = m.and_then(|m| m.tiers.iter().find(|t| Some(t.variant.as_str()) == variant));
    let bonus = variant
        .filter(|_| m.is_some_and(|m| m.variant_bonus))
        .and_then(|v| v.parse::<i32>().ok());
    let stem = name.split(',').next().unwrap_or(&name).trim().to_owned();
    let ammunition = rec["subtype"] == "Any Ammunition";

    let mut item = match base {
        Some(base) if ammunition => Item {
            name: base.to_owned(),
            kind: ItemKind::Ammunition,
            ..Default::default()
        },
        Some(base) => {
            let rec = found(base)?;
            match rec["category"].as_str() {
                Some("armor") => armor_item(&rec),
                _ => weapon_item(&rec),
            }
        }
        None => Item {
            kind: match rec["item_type"].as_str() {
                Some("potion" | "scroll") => ItemKind::Consumable,
                _ => ItemKind::Gear,
            },
            ..Default::default()
        },
    };
    let base_name = std::mem::take(&mut item.name);
    item.name = match (tier, variant, base) {
        (Some(tier), _, _) => tier.name.clone(),
        (_, Some(v), Some(_)) => format!("{v} {base_name}"),
        (_, Some(v), None) => format!("{v} {stem}"),
        (_, None, Some(_)) if open.bases.len() > 1 => format!("{name} ({base_name})"),
        _ => name,
    };
    item.notes = text(&rec["markdown"]);
    item.requires_attunement = rec["attunement"]["required"] == true;

    if let Some(max) = rec["charges"]["max"].as_u64() {
        item.charges_max = max as u32;
        if let Some(regain) = rec["charges"]["regain"].as_str() {
            item.recharge = Recharge::Dawn;
            item.regain = regain.split_whitespace().collect();
        }
        derived.push(format!("{max} charges"));
    }
    if rec["item_type"] == "potion" && m.is_none_or(|m| m.actions.is_empty()) {
        let oil = rec["name"].as_str().is_some_and(|n| n.starts_with("Oil"));
        item.actions.push(Action {
            name: if oil { "Apply" } else { "Drink" }.into(),
            activation: if oil {
                Activation::Other
            } else {
                Activation::BonusAction
            },
            cost: Cost::Quantity,
            roll: tier.map(|t| t.roll.clone()).unwrap_or_default(),
            roll_effect: m.map(|m| m.roll_effect).unwrap_or_default(),
            ..Default::default()
        });
    }
    if let Some(m) = m {
        if m.charges_max > 0 {
            item.charges_max = m.charges_max;
            item.recharge = m.recharge;
            item.regain = m.regain.clone();
            derived.push(format!("{} charges", m.charges_max));
        }
        item.modifiers = chosen_modifiers(&m.modifiers, choice);
        item.actions.extend(chosen_actions(&m.actions, choice));
        item.ac_bonus = m.ac_bonus;
        if m.magic_bonus != 0 || !m.damage_type.is_empty() || !m.as_weapon.is_empty() {
            if item.weapon.is_some() {
                item.kind = ItemKind::Weapon;
            }
            if let Some(w) = &mut item.weapon {
                w.magic_bonus = m.magic_bonus;
                if !m.damage_type.is_empty() {
                    w.damage_type = m.damage_type.clone();
                }
            }
        }
    }
    if let Some(n) = bonus {
        match item.kind {
            ItemKind::Weapon => {
                if let Some(w) = &mut item.weapon {
                    w.magic_bonus = n;
                }
            }
            ItemKind::Ammunition => {
                for kind in [ModifierKind::AttackRoll, ModifierKind::Damage] {
                    item.modifiers.push(Modifier {
                        kind,
                        value: n,
                        ..Default::default()
                    });
                }
            }
            _ => item.ac_bonus = n,
        }
    }
    derived.extend(item.modifiers.iter().map(Modifier::describe));
    Ok(item)
}

/// The short line that opens a spell's notes, as a row shows only the first paragraph.
fn spell_facts(rec: &Value) -> String {
    let mut facts = Vec::new();
    if rec["casting_time"]["activation"] == "time" {
        facts.push(text(&rec["casting_time"]["text"]));
    }
    facts.push(text(&rec["range"]["text"]));
    facts.push(text(&rec["duration"]["text"]));
    let components = text(&rec["components"]["text"]);
    facts.push(components.split(" (").next().unwrap_or_default().to_owned());
    for save in rec["saves"].as_array().map_or(&[][..], Vec::as_slice) {
        let ability = text(save).to_uppercase();
        facts.push(format!("{} save", &ability[..3.min(ability.len())]));
    }
    if rec["attack"].is_string() {
        facts.push(format!("{} spell attack", text(&rec["attack"])));
    }
    facts.join(" · ")
}

/// A record's prose without the header lines that repeat its fields.
fn spell_prose(rec: &Value) -> String {
    let markdown = text(&rec["markdown"]);
    let body = markdown
        .split_once("**Duration:**")
        .and_then(|(_, rest)| rest.split_once("\n\n"))
        .map_or(markdown.as_str(), |(_, body)| body);
    body.trim().to_owned()
}

/// Rounds for a spell's effect, or 0 for one that outlasts combat tracking.
fn spell_rounds(rec: &Value) -> u32 {
    let d = &rec["duration"];
    let amount = d["amount"].as_u64().unwrap_or(0) as u32;
    match d["unit"].as_str() {
        Some("round") => amount,
        Some("minute") if amount == 1 => 10,
        _ => 0,
    }
}

/// The dice a spell's structured fields make certain: a single damage roll from an attack or
/// a save that's over at once. Anything else may be a rider or a later effect.
fn spell_dice(rec: &Value) -> Option<&str> {
    let damage = spell_damage(rec)?;
    let instant_save = rec["saves"].is_array() && rec["duration"]["kind"] == "instantaneous";
    let certain = rec["concentration"] == false && (rec["attack"].is_string() || instant_save);
    damage["dice"].as_str().filter(|_| certain)
}

/// The spell's damage entry, if it has exactly one.
fn spell_damage(rec: &Value) -> Option<&Value> {
    match rec["damage"].as_array()?.as_slice() {
        [damage] => Some(damage),
        _ => None,
    }
}

/// `{cantrip}d10` for a cantrip that prints its dice at each tier, such as `(2d10)`.
fn cantrip_formula(rec: &Value, dice: &str) -> Option<String> {
    let upgrade = rec["cantrip_upgrade"].as_str()?;
    let (count, die) = dice.split_once('d')?;
    let tiers = [2, 3, 4].map(|n| format!("({n}d{die})"));
    (count == "1" && tiers.iter().all(|t| upgrade.contains(t.as_str())))
        .then(|| format!("{{cantrip}}d{die}"))
}

fn spell(rec: &Value, m: Option<&Mechanics>, derived: &mut Vec<String>) -> Spell {
    let concentration = rec["concentration"] == true;
    let mut spell = Spell {
        name: name_of(rec),
        level: rec["level"].as_u64().unwrap_or(0) as u8,
        ritual: rec["ritual"] == true,
        concentration,
        notes: format!("{}\n\n{}", spell_facts(rec), spell_prose(rec)),
        casting_time: match rec["casting_time"]["activation"].as_str() {
            Some("action") => Activation::Action,
            Some("bonus_action") => Activation::BonusAction,
            Some("reaction") => Activation::Reaction,
            _ => Activation::Other,
        },
        ..Default::default()
    };
    if let Some(dice) = spell_dice(rec) {
        match cantrip_formula(rec, dice) {
            Some(formula) => spell.roll_formula = formula,
            None => spell.roll = dice.to_owned(),
        }
        if let Some(kind) = spell_damage(rec).and_then(|d| d["type"].as_str()) {
            spell.damage_type = kind.to_owned();
        }
    }
    if let Some(m) = m {
        if !m.roll.is_empty() {
            spell.roll = m.roll.clone();
        }
        if !m.roll_formula.is_empty() {
            spell.roll_formula = m.roll_formula.clone();
        }
        spell.roll_effect = m.roll_effect;
    }
    let curated = m.and_then(|m| m.effect.as_ref());
    if concentration || curated.is_some() {
        spell.effect = Some(Effect {
            name: spell.name.clone(),
            source: "Spell".into(),
            concentration,
            rounds: curated.map_or_else(|| spell_rounds(rec), |e| e.rounds),
            modifiers: curated.map_or_else(Vec::new, |e| e.modifiers.clone()),
            notes: String::new(),
        });
    }
    if !spell.roll.is_empty() || !spell.roll_formula.is_empty() {
        let dice = [&spell.roll_formula, &spell.roll];
        derived.push(format!(
            "Rolls {}",
            dice.iter().find(|d| !d.is_empty()).unwrap()
        ));
    }
    if let Some(e) = &spell.effect {
        derived.extend(e.modifiers.iter().map(Modifier::describe));
    }
    spell
}

fn feature(
    rec: &Value,
    choice: Option<&str>,
    m: Option<&Mechanics>,
    derived: &mut Vec<String>,
) -> Feature {
    let mut feature = Feature {
        name: name_of(rec),
        source: match rec["category"].as_str() {
            Some("feat") => "Feat".to_owned(),
            _ => text(&rec["source"]),
        },
        description: text(&rec["markdown"]),
        ..Default::default()
    };
    if let Some(m) = m {
        feature.uses_max = m.uses_max;
        feature.uses_formula = m.uses_formula.clone();
        feature.recharge = m.recharge;
        feature.modifiers = chosen_modifiers(&m.modifiers, choice);
        feature.actions = chosen_actions(&m.actions, choice);
        derived.extend(feature.modifiers.iter().map(Modifier::describe));
        if !m.uses_formula.is_empty() {
            derived.push(format!("Uses follow {}", m.uses_formula));
        }
    }
    feature
}

/// Whether a new item of this kind can go straight into use without displacing another.
fn free_slot(c: &Character, item: &Item) -> bool {
    match item.kind {
        ItemKind::Armor | ItemKind::Shield => !c
            .inventory
            .iter()
            .any(|i| i.kind == item.kind && i.equipped),
        ItemKind::Weapon => true,
        ItemKind::Gear => !item.modifiers.is_empty(),
        _ => false,
    }
}

fn uses_spell_modifier(formula: &str) -> bool {
    formula.contains("{spell}")
}

/// Makes the entry a record describes for this character, without adding it.
pub fn convert(spec: &AddSpec, c: &Character) -> Result<Conversion, SrdError> {
    let rec = found(&spec.id)?;
    let category = text(&rec["category"]);
    let m = mechanics::of(&spec.id);
    let mut derived = Vec::new();
    let mut warnings = Vec::new();
    let mut item = match category.as_str() {
        "weapon" => weapon_item(&rec),
        "armor" => armor_item(&rec),
        "gear" => gear_item(&rec, spec)?,
        "tool" => tool_item(&rec),
        "magic_item" => magic_item(&rec, spec, m, &mut derived)?,
        "spell" => {
            let mut spell = spell(&rec, m, &mut derived);
            spell.prepared = spec.prepared.unwrap_or(true);
            if uses_spell_modifier(&spell.roll_formula) && c.spellcasting_abilities().len() > 1 {
                warnings.push(format!(
                    "{} uses the spellcasting modifier of your highest-level casting class",
                    spell.name
                ));
            }
            return Ok(Conversion {
                entry: Entry::Spell(Box::new(spell)),
                derived,
                warnings,
            });
        }
        "feat" | "species_trait" | "class_feature" | "subclass_feature" | "class_option" => {
            let allowed = m
                .and_then(|m| m.choice.as_ref())
                .map_or(&[][..], |c| &c.allowed[..]);
            let name = name_of(&rec);
            let choice = pick(&name, "choice", spec.choice.as_deref(), allowed)?;
            return Ok(Conversion {
                entry: Entry::Feature(Box::new(feature(&rec, choice, m, &mut derived))),
                derived,
                warnings,
            });
        }
        "background" => {
            return Err(SrdError::NotAddable(
                name_of(&rec),
                "add its feat and record its skills on the sheet",
            ));
        }
        "species" | "class" | "subclass" => {
            return Err(SrdError::NotAddable(
                name_of(&rec),
                "add its traits or features one at a time",
            ));
        }
        _ => {
            return Err(SrdError::NotAddable(
                name_of(&rec),
                "look its rules up instead",
            ));
        }
    };
    if let Some(quantity) = spec.quantity {
        item.quantity = quantity;
    }
    item.equipped = spec.equipped.unwrap_or_else(|| free_slot(c, &item));
    item.attuned = item.requires_attunement && spec.attuned.unwrap_or(false);
    if !item.equipped && matches!(item.kind, ItemKind::Armor | ItemKind::Shield) {
        warnings.push(format!("{} isn't equipped yet", item.name));
    }
    if item.requires_attunement && !item.attuned {
        warnings.push(format!("{} does nothing until it's attuned", item.name));
    }
    if let Some(w) = &item.weapon {
        derived.insert(0, format!("{} {} damage", w.damage, w.damage_type));
    }
    Ok(Conversion {
        entry: Entry::Item(Box::new(item)),
        derived,
        warnings,
    })
}

/// What adding a record did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Added {
    /// Where the entry is.
    pub pointer: String,
    pub entry: Value,
    pub derived: Vec<String>,
    pub warnings: Vec<String>,
}

/// Adds a record to a character. An item already carried is stacked, and a spell or feature
/// already on the sheet is refused unless the record is repeatable.
pub fn add(c: &mut Character, spec: &AddSpec) -> Result<Added, SrdError> {
    let Conversion {
        entry,
        derived,
        mut warnings,
    } = convert(spec, c)?;
    let repeatable = found(&spec.id)?["repeatable"] == true;
    let (pointer, value) = match &entry {
        Entry::Item(item) => {
            let held = c
                .inventory
                .iter()
                .position(|i| i.name == item.name && i.kind == item.kind);
            match held {
                Some(i) => {
                    let quantity = c.inventory[i].quantity + item.quantity;
                    let pointer = format!("/inventory/{i}");
                    edit::set(c, &format!("{pointer}/quantity"), json!(quantity))?;
                    warnings.push(format!("You now have {quantity} of {}", item.name));
                    (pointer, json!(c.inventory[i]))
                }
                None => {
                    edit::push(c, "/inventory", json!(item))?;
                    let i = c.inventory.len() - 1;
                    (format!("/inventory/{i}"), json!(c.inventory[i]))
                }
            }
        }
        Entry::Spell(spell) => {
            if c.spells.iter().any(|s| s.name == spell.name) {
                return Err(SrdError::Duplicate(spell.name.clone()));
            }
            edit::push(c, "/spells", json!(spell))?;
            let i = c.spells.len() - 1;
            (format!("/spells/{i}"), json!(c.spells[i]))
        }
        Entry::Feature(feature) => {
            if !repeatable && c.features.iter().any(|f| f.name == feature.name) {
                return Err(SrdError::Duplicate(feature.name.clone()));
            }
            edit::push(c, "/features", json!(feature))?;
            let i = c.features.len() - 1;
            (format!("/features/{i}"), json!(c.features[i]))
        }
    };
    Ok(Added {
        pointer,
        entry: value,
        derived,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::super::data::records;
    use super::*;
    use crate::abilities::Ability;
    use crate::dice::Expr;
    use crate::effects::RollEffect;

    fn spec(id: &str) -> AddSpec {
        AddSpec {
            id: id.into(),
            ..Default::default()
        }
    }

    fn item(id: &str, c: &Character) -> Item {
        match convert(&spec(id), c).unwrap().entry {
            Entry::Item(item) => *item,
            other => panic!("{id} isn't an item: {other:?}"),
        }
    }

    fn built(s: &AddSpec) -> Item {
        match convert(s, &fresh()).unwrap().entry {
            Entry::Item(item) => *item,
            other => panic!("{other:?}"),
        }
    }

    fn cast(id: &str) -> Spell {
        match convert(&spec(id), &fresh()).unwrap().entry {
            Entry::Spell(spell) => *spell,
            other => panic!("{other:?}"),
        }
    }

    fn feat(id: &str) -> Feature {
        match convert(&spec(id), &fresh()).unwrap().entry {
            Entry::Feature(feature) => *feature,
            other => panic!("{other:?}"),
        }
    }

    fn fresh() -> Character {
        Character::new("A")
    }

    #[test]
    fn every_weapon_converts_to_rollable_stats() {
        for rec in records("weapons.json") {
            let i = item(rec["id"].as_str().unwrap(), &fresh());
            let w = i.weapon.unwrap();
            assert_eq!(i.kind, ItemKind::Weapon);
            Expr::parse(&w.damage).unwrap();
            if !w.versatile.is_empty() {
                Expr::parse(&w.versatile).unwrap();
            }
            assert!(!w.damage_type.is_empty());
            assert!(!w.mastery.is_empty(), "{} has no mastery", i.name);
        }
    }

    #[test]
    fn longsword_keeps_its_properties() {
        let i = item("weapon:longsword", &fresh());
        assert!(i.equipped);
        assert_eq!(i.weight, 3.0);
        let w = i.weapon.unwrap();
        assert_eq!(w.damage, "1d8");
        assert_eq!(w.versatile, "1d10");
        assert_eq!(w.mastery, "Sap");
        assert_eq!(w.properties, ["Versatile (1d10)"]);
    }

    #[test]
    fn ranged_and_thrown_weapons() {
        let bow = item("weapon:longbow", &fresh()).weapon.unwrap();
        assert!(bow.ranged);
        assert_eq!(bow.range, "150/600");
        let dagger = item("weapon:dagger", &fresh()).weapon.unwrap();
        assert!(dagger.finesse && dagger.light && dagger.thrown && !dagger.ranged);
        assert_eq!(dagger.range, "20/60");
    }

    #[test]
    fn masteries_are_the_apps_options() {
        let names: Vec<String> = records("mastery_properties.json")
            .iter()
            .map(name_of)
            .collect();
        for option in [
            "Cleave", "Graze", "Nick", "Push", "Sap", "Slow", "Topple", "Vex",
        ] {
            assert!(names.iter().any(|n| n == option));
        }
    }

    #[test]
    fn every_armor_matches_the_apps_table() {
        for rec in records("armor.json") {
            let id = rec["id"].as_str().unwrap();
            let i = item(id, &fresh());
            if id == "armor:shield" {
                assert_eq!(i.kind, ItemKind::Shield);
                assert_eq!(i.armor, None);
                continue;
            }
            let info = i.armor.unwrap_or_else(|| panic!("{id} has no kind")).info();
            assert_eq!(info.base_ac as i64, rec["ac"]["base"].as_i64().unwrap());
            assert_eq!(
                info.stealth_disadvantage,
                rec["stealth_disadvantage"].as_bool().unwrap()
            );
        }
    }

    #[test]
    fn adding_armor_equips_it_but_a_second_suit_waits() {
        let mut c = fresh();
        let before = c.armor_class();
        add(&mut c, &spec("armor:shield")).unwrap();
        assert_eq!(c.armor_class(), before + 2);
        add(&mut c, &spec("armor:chain-mail")).unwrap();
        let second = add(&mut c, &spec("armor:plate-armor")).unwrap();
        assert!(c.inventory[1].equipped);
        assert!(!c.inventory[2].equipped);
        assert_eq!(second.pointer, "/inventory/2");
        assert_eq!(second.warnings.len(), 1);
    }

    #[test]
    fn ammunition_needs_a_variant_and_counts_pieces() {
        let bare = convert(&spec("gear:ammunition"), &fresh()).unwrap_err();
        assert!(bare.to_string().contains("Arrows"));
        let arrows = AddSpec {
            variant: Some("Arrows".into()),
            ..spec("gear:ammunition")
        };
        let i = built(&arrows);
        assert_eq!(i.kind, ItemKind::Ammunition);
        assert_eq!(i.quantity, 20);
        assert_eq!(i.weight, 0.05);
    }

    #[test]
    fn gear_and_tools_keep_their_text() {
        let acid = item("gear:acid", &fresh());
        assert_eq!(acid.kind, ItemKind::Consumable);
        assert!(acid.notes.contains("2d6 Acid damage"));
        assert!(acid.actions.is_empty() && acid.modifiers.is_empty());
        let tools = item("tool:thieves-tools", &fresh());
        assert_eq!(tools.kind, ItemKind::Gear);
        assert!(tools.notes.contains("Pick a lock"));
    }

    #[test]
    fn every_gear_and_tool_record_converts() {
        for file in ["gear.json", "tools.json"] {
            for rec in records(file)
                .iter()
                .filter(|r| r["id"] != "gear:ammunition")
            {
                convert(&spec(rec["id"].as_str().unwrap()), &fresh()).unwrap();
            }
        }
    }

    #[test]
    fn lookup_only_records_are_refused() {
        let e = convert(&spec("condition:prone"), &fresh()).unwrap_err();
        assert!(matches!(e, SrdError::NotAddable(..)));
        assert!(matches!(
            convert(&spec("spell:nope"), &fresh()),
            Err(SrdError::NotFound(_))
        ));
    }

    #[test]
    fn every_spell_converts_with_rollable_dice() {
        let mut structured = 0;
        for rec in records("spells.json") {
            let spell = cast(rec["id"].as_str().unwrap());
            assert!(!spell.notes.starts_with('*'), "{}", spell.name);
            if !spell.roll.is_empty() {
                Expr::parse(&spell.roll).unwrap();
            }
            structured += usize::from(spell_dice(rec).is_some());
        }
        assert_eq!(structured, 33);
    }

    #[test]
    fn fireball_rolls_its_damage_and_opens_with_its_facts() {
        let s = cast("spell:fireball");
        assert_eq!((s.roll.as_str(), s.level), ("8d6", 3));
        assert!(s.prepared);
        let first = s.notes.split("\n\n").next().unwrap();
        assert_eq!(first, "150 feet · Instantaneous · V, S, M · DEX save");
        assert!(s.notes.contains("Using a Higher-Level Spell Slot"));
    }

    #[test]
    fn cantrips_scale_only_where_their_dice_are_printed() {
        assert_eq!(cast("spell:fire-bolt").roll_formula, "{cantrip}d10");
        let blast = cast("spell:eldritch-blast");
        assert_eq!(
            (blast.roll.as_str(), blast.roll_formula.as_str()),
            ("1d10", "")
        );
    }

    #[test]
    fn rollable_spells_know_their_damage_type() {
        for rec in records("spells.json") {
            let spell = spell(rec, None, &mut Vec::new());
            if !spell.roll.is_empty() || !spell.roll_formula.is_empty() {
                assert_ne!(spell.damage_type, "", "{}", spell.name);
            }
        }
    }

    #[test]
    fn spells_without_certain_dice_roll_nothing() {
        for id in [
            "spell:teleport",
            "spell:bless",
            "spell:acid-arrow",
            "spell:mage-armor",
        ] {
            assert_eq!(cast(id).roll, "", "{id}");
        }
    }

    #[test]
    fn concentration_spells_track_concentration() {
        let mut c = fresh();
        c.classes[0].class = crate::classes::ClassId::Cleric;
        c.classes[0].level = 5;
        add(&mut c, &spec("spell:bless")).unwrap();
        let e = c.spells[0].effect.as_ref().unwrap();
        assert!(e.concentration);
        assert_eq!(e.rounds, 10);
        assert!(
            crate::view::SheetView::new(&c)
                .derived
                .actions
                .iter()
                .any(|a| a.name == "Bless")
        );
        c.cast_spell(0, 1).unwrap();
        assert_eq!(c.effects.len(), 1);
    }

    #[test]
    fn long_concentration_outlasts_rounds() {
        assert_eq!(cast("spell:fly").effect.unwrap().rounds, 0);
        assert_eq!(cast("spell:mirror-image").effect, None);
    }

    #[test]
    fn healing_follows_the_casting_modifier() {
        let mut c = fresh();
        c.classes[0].class = crate::classes::ClassId::Cleric;
        c.base_scores[Ability::Wis.index()] = 16;
        add(&mut c, &spec("spell:cure-wounds")).unwrap();
        assert_eq!(c.spells[0].roll, "2d8+3");
        assert_eq!(c.spells[0].roll_effect, RollEffect::None);
        edit::set(&mut c, "/base_scores/4", json!(18)).unwrap();
        assert_eq!(c.spells[0].roll, "2d8+4");
    }

    #[test]
    fn a_multiclass_caster_is_warned_about_the_assumed_modifier() {
        let mut c = fresh();
        c.classes[0].class = crate::classes::ClassId::Cleric;
        c.classes.push(crate::character::ClassLevel {
            class: crate::classes::ClassId::Wizard,
            level: 1,
            ..Default::default()
        });
        let added = add(&mut c, &spec("spell:cure-wounds")).unwrap();
        assert_eq!(added.warnings.len(), 1);
    }

    #[test]
    fn self_effects_apply_when_cast() {
        let mut c = fresh();
        c.classes[0].class = crate::classes::ClassId::Wizard;
        let before = c.armor_class();
        add(&mut c, &spec("spell:shield")).unwrap();
        c.cast_spell(0, 1).unwrap();
        assert_eq!(c.armor_class(), before + 5);
    }

    fn plus_one(base: &str) -> AddSpec {
        AddSpec {
            base: Some(base.into()),
            variant: Some("+1".into()),
            ..spec("magic_item:weapon-plus-1-plus-2-or-plus-3")
        }
    }

    #[test]
    fn every_subtype_resolves_to_a_base() {
        for rec in records("magic_items.json") {
            if rec["subtype"].is_string() {
                let id = rec["id"].as_str().unwrap();
                assert!(!options(id).unwrap().bases.is_empty(), "{id}");
            }
        }
    }

    #[test]
    fn a_plus_one_longsword_hits_harder() {
        let mut c = fresh();
        c.base_scores[Ability::Str.index()] = 16;
        add(&mut c, &plus_one("weapon:longsword")).unwrap();
        assert_eq!(c.inventory[0].name, "+1 Longsword");
        let attacks = c.weapon_attacks();
        assert_eq!(attacks[0].to_hit, 3 + 2 + 1);
        assert_eq!(attacks[0].damage, "1d8+4");
        assert_eq!(attacks[1].name, "+1 Longsword (two-handed)");
        assert_eq!(attacks[1].damage, "1d10+4");
    }

    #[test]
    fn missing_options_say_what_is_allowed() {
        let bare = AddSpec {
            base: None,
            ..plus_one("")
        };
        let error = convert(&bare, &fresh()).unwrap_err().to_string();
        assert!(
            error.contains("base") && error.contains("weapon:longsword"),
            "{error}"
        );
        let wrong = plus_one("weapon:nope");
        assert!(convert(&wrong, &fresh()).is_err());
    }

    #[test]
    fn a_fixed_base_is_selected_alone() {
        let i = item("magic_item:dagger-of-venom", &fresh());
        assert_eq!(i.name, "Dagger of Venom");
        assert_eq!(i.weapon.unwrap().magic_bonus, 1);
    }

    #[test]
    fn named_weapons_keep_their_base_in_the_name() {
        let tongue = AddSpec {
            base: Some("weapon:longsword".into()),
            ..spec("magic_item:flame-tongue")
        };
        let i = built(&tongue);
        assert_eq!(i.name, "Flame Tongue (Longsword)");
        assert!(i.requires_attunement);
        assert!(i.modifiers[0].toggleable && !i.modifiers[0].enabled);
    }

    #[test]
    fn staffs_can_be_wielded() {
        let i = item("magic_item:staff-of-striking", &fresh());
        assert_eq!(i.kind, ItemKind::Weapon);
        assert_eq!(i.weapon.as_ref().unwrap().magic_bonus, 3);
        assert_eq!(i.weapon.unwrap().damage, "1d6");
    }

    #[test]
    fn magic_armor_and_shields_add_to_ac() {
        let plate = AddSpec {
            base: Some("armor:plate-armor".into()),
            variant: Some("+2".into()),
            ..spec("magic_item:armor-plus-1-plus-2-or-plus-3")
        };
        let i = built(&plate);
        assert_eq!((i.name.as_str(), i.ac_bonus), ("+2 Plate Armor", 2));
        assert_eq!(i.armor, Some(ArmorKind::Plate));

        let shield = AddSpec {
            variant: Some("+1".into()),
            ..spec("magic_item:shield-plus-1-plus-2-or-plus-3")
        };
        let mut c = fresh();
        let before = c.armor_class();
        add(&mut c, &shield).unwrap();
        assert_eq!(c.inventory[0].name, "+1 Shield");
        assert_eq!(c.armor_class(), before + 3);
    }

    #[test]
    fn magic_ammunition_carries_the_bonus() {
        let arrows = AddSpec {
            base: Some("Arrows".into()),
            variant: Some("+2".into()),
            quantity: Some(10),
            ..spec("magic_item:ammunition-plus-1-plus-2-or-plus-3")
        };
        let i = built(&arrows);
        assert_eq!((i.name.as_str(), i.quantity), ("+2 Arrows", 10));
        assert_eq!(i.kind, ItemKind::Ammunition);
        assert_eq!(i.modifiers.len(), 2);
    }

    #[test]
    fn a_cloak_does_nothing_until_attuned() {
        let mut c = fresh();
        let before = c.armor_class();
        let added = add(&mut c, &spec("magic_item:cloak-of-protection")).unwrap();
        assert!(added.warnings[0].contains("attuned"));
        assert_eq!(c.armor_class(), before);
        edit::set(&mut c, "/inventory/0/attuned", json!(true)).unwrap();
        assert_eq!(c.armor_class(), before + 1);
    }

    #[test]
    fn potion_tiers_heal_their_own_dice() {
        let greater = AddSpec {
            variant: Some("Greater".into()),
            ..spec("magic_item:potions-of-healing")
        };
        let i = built(&greater);
        assert_eq!(i.name, "Potion of Greater Healing");
        assert_eq!(i.kind, ItemKind::Consumable);
        assert_eq!(i.actions[0].roll, "4d4+4");
        assert_eq!(i.actions[0].roll_effect, RollEffect::Heal);
        assert_eq!(i.actions[0].cost, Cost::Quantity);
    }

    #[test]
    fn other_potions_are_drunk_with_no_invented_effect() {
        let i = item("magic_item:potion-of-flying", &fresh());
        assert_eq!(i.actions.len(), 1);
        assert_eq!(i.actions[0].name, "Drink");
        assert!(i.actions[0].roll.is_empty() && i.actions[0].effect.is_none());
        let oil = item("magic_item:oil-of-slipperiness", &fresh());
        assert_eq!(oil.actions[0].activation, Activation::Other);
    }

    #[test]
    fn a_chosen_damage_type_fills_the_resistance() {
        let bare = convert(&spec("magic_item:ring-of-resistance"), &fresh());
        assert!(matches!(bare, Err(SrdError::NeedsOption { .. })));
        let fire = AddSpec {
            choice: Some("fire".into()),
            ..spec("magic_item:ring-of-resistance")
        };
        assert_eq!(built(&fire).modifiers[0].damage_type, "fire");
    }

    #[test]
    fn charges_regain_at_dawn() {
        let wand = item("magic_item:wand-of-magic-missiles", &fresh());
        assert_eq!(wand.recharge, Recharge::Dawn);
        assert!(wand.charges_max > 0);
        Expr::parse(&wand.regain).unwrap();
    }

    #[test]
    fn every_regain_string_rolls() {
        for rec in records("magic_items.json") {
            if let Some(regain) = rec["charges"]["regain"].as_str() {
                let dice: String = regain.split_whitespace().collect();
                Expr::parse(&dice).unwrap_or_else(|e| panic!("{regain}: {e}"));
            }
        }
    }

    #[test]
    fn records_outside_the_table_get_nothing_invented() {
        for rec in records("magic_items.json") {
            let id = rec["id"].as_str().unwrap();
            if mechanics::of(id).is_some() {
                continue;
            }
            let open = options(id).unwrap();
            let s = AddSpec {
                base: open.bases.first().map(|b| b.id.clone()),
                variant: open.variants.first().cloned(),
                ..spec(id)
            };
            let i = built(&s);
            assert!(i.modifiers.is_empty(), "{id} got modifiers");
            assert_eq!(i.ac_bonus, 0, "{id}");
            assert!(i.weapon.is_none_or(|w| w.magic_bonus == 0), "{id}");
            assert!(
                i.actions
                    .iter()
                    .all(|a| a.roll.is_empty() && a.effect.is_none()),
                "{id}"
            );
        }
        for file in ["feats.json", "species.json"] {
            for rec in records(file) {
                for part in std::iter::once(rec.clone()).chain(super::super::data::parts(rec)) {
                    let id = part["id"].as_str().unwrap();
                    if mechanics::of(id).is_none() && convert(&spec(id), &fresh()).is_ok() {
                        let f = feat(id);
                        assert!(f.modifiers.is_empty() && f.actions.is_empty(), "{id}");
                        assert_eq!(f.uses_max, 0, "{id}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_mechanics_entry_converts() {
        for (id, m) in mechanics::all() {
            let open = options(id).unwrap();
            let s = AddSpec {
                base: open.bases.first().map(|b| b.id.clone()),
                variant: open.variants.first().cloned(),
                choice: m.choice.as_ref().map(|c| c.allowed[0].clone()),
                ..spec(id)
            };
            let c = convert(&s, &fresh()).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert!(!format!("{:?}", c.entry).contains("{choice}"), "{id}");
        }
    }

    #[test]
    fn traits_and_feats_become_features() {
        let f = feat("species:dwarf#dwarven-resilience");
        assert_eq!(f.source, "Dwarf");
        assert_eq!(f.modifiers[0].damage_type, "poison");
        let f = feat("class:barbarian#rage");
        assert_eq!(f.source, "Barbarian 1");
        assert!(f.modifiers.is_empty() && f.uses_max == 0);
        let f = feat("feat:ability-score-improvement");
        assert_eq!(f.source, "Feat");
        assert!(f.description.contains("Increase one ability score"));
    }

    #[test]
    fn scaling_traits_follow_the_character() {
        let mut c = fresh();
        add(&mut c, &spec("species:dwarf#stonecunning")).unwrap();
        add(&mut c, &spec("species:dwarf#dwarven-toughness")).unwrap();
        add(&mut c, &spec("feat:alert")).unwrap();
        let hp = c.max_hp();
        let init = c.initiative();
        assert_eq!(c.features[0].uses_max, 2);
        edit::set(&mut c, "/classes/0/level", json!(9)).unwrap();
        assert_eq!(c.features[0].uses_max, 4);
        assert_eq!(c.max_hp(), hp + 8);
        assert_eq!(c.initiative(), init + 2);
    }

    #[test]
    fn duplicates_stack_or_are_refused() {
        let mut c = fresh();
        add(&mut c, &spec("gear:rope")).unwrap();
        let again = add(&mut c, &spec("gear:rope")).unwrap();
        assert_eq!(again.pointer, "/inventory/0");
        assert_eq!(c.inventory[0].quantity, 2);

        add(&mut c, &spec("spell:fireball")).unwrap();
        let spell = add(&mut c, &spec("spell:fireball"));
        assert!(matches!(spell, Err(SrdError::Duplicate(_))));

        add(&mut c, &spec("feat:skilled")).unwrap();
        add(&mut c, &spec("feat:skilled")).unwrap();
        assert_eq!(c.features.len(), 2);
        add(&mut c, &spec("feat:alert")).unwrap();
        assert!(add(&mut c, &spec("feat:alert")).is_err());
    }

    #[test]
    fn whole_species_and_backgrounds_are_refused_with_a_hint() {
        let e = convert(&spec("species:dwarf"), &fresh()).unwrap_err();
        assert!(e.to_string().contains("traits"));
        let e = convert(&spec("background:acolyte"), &fresh()).unwrap_err();
        assert!(e.to_string().contains("feat"));
    }
}
