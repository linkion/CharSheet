//! Turning SRD records into sheet entries. Only structured fields are read here: prose is
//! copied across for the player, never interpreted.

use super::data::record;
use crate::armor::ArmorKind;
use crate::character::{Character, Item, ItemKind, WeaponStats};
use crate::edit::{self, EditError};
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
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Conversion {
    pub entry: Entry,
    /// What was filled in beyond the text, for review.
    pub derived: Vec<String>,
    /// What the entry won't do until the player acts, or may get wrong.
    pub warnings: Vec<String>,
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

/// Whether a new item of this kind can go straight into use without displacing another.
fn free_slot(c: &Character, item: &Item) -> bool {
    match item.kind {
        ItemKind::Armor | ItemKind::Shield => !c
            .inventory
            .iter()
            .any(|i| i.kind == item.kind && i.equipped),
        ItemKind::Weapon => true,
        _ => false,
    }
}

/// Makes the entry a record describes for this character, without adding it.
pub fn convert(spec: &AddSpec, c: &Character) -> Result<Conversion, SrdError> {
    let rec = found(&spec.id)?;
    let category = text(&rec["category"]);
    let mut warnings = Vec::new();
    let mut item = match category.as_str() {
        "weapon" => weapon_item(&rec),
        "armor" => armor_item(&rec),
        "gear" => gear_item(&rec, spec)?,
        "tool" => tool_item(&rec),
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
    if !item.equipped && matches!(item.kind, ItemKind::Armor | ItemKind::Shield) {
        warnings.push(format!("{} isn't equipped yet", item.name));
    }
    let derived = match &item.weapon {
        Some(w) => vec![format!("{} {} damage", w.damage, w.damage_type)],
        None => Vec::new(),
    };
    Ok(Conversion {
        entry: Entry::Item(Box::new(item)),
        derived,
        warnings,
    })
}

/// What adding a record did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Added {
    /// Where the new entry is.
    pub pointer: String,
    pub entry: Value,
    pub derived: Vec<String>,
    pub warnings: Vec<String>,
}

/// Adds a record to a character.
pub fn add(c: &mut Character, spec: &AddSpec) -> Result<Added, SrdError> {
    let Conversion {
        entry,
        derived,
        warnings,
    } = convert(spec, c)?;
    let (list, value) = match &entry {
        Entry::Item(item) => ("/inventory", json!(item)),
    };
    edit::push(c, list, value.clone())?;
    let pointer = format!("{list}/{}", c.inventory.len() - 1);
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
    use crate::dice::Expr;

    fn spec(id: &str) -> AddSpec {
        AddSpec {
            id: id.into(),
            ..Default::default()
        }
    }

    fn item(id: &str, c: &Character) -> Item {
        let Entry::Item(item) = convert(&spec(id), c).unwrap().entry;
        *item
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
    fn adding_a_shield_raises_ac_but_a_second_stays_unequipped() {
        let mut c = fresh();
        let before = c.armor_class();
        add(&mut c, &spec("armor:shield")).unwrap();
        assert_eq!(c.armor_class(), before + 2);
        let second = add(&mut c, &spec("armor:shield")).unwrap();
        assert!(!c.inventory[1].equipped);
        assert_eq!(c.armor_class(), before + 2);
        assert_eq!(second.pointer, "/inventory/1");
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
        let Entry::Item(i) = convert(&arrows, &fresh()).unwrap().entry;
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
}
