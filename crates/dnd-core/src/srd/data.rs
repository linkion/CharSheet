//! The embedded JSON files, each parsed the first time it's needed.

use super::slug;
use serde_json::{Value, json};
use std::sync::OnceLock;

macro_rules! file {
    ($name:literal) => {
        ($name, include_str!(concat!("../../../../data/srd/", $name)))
    };
}

const FILES: [(&str, &str); 20] = [
    file!("armor.json"),
    file!("backgrounds.json"),
    file!("classes.json"),
    file!("conditions.json"),
    file!("feats.json"),
    file!("gear.json"),
    file!("glossary.json"),
    file!("index.json"),
    file!("magic_items.json"),
    file!("mastery_properties.json"),
    file!("monsters.json"),
    file!("mounts_vehicles.json"),
    file!("rules.json"),
    file!("services.json"),
    file!("species.json"),
    file!("spells.json"),
    file!("subclasses.json"),
    file!("tools.json"),
    file!("weapon_properties.json"),
    file!("weapons.json"),
];

const MANIFEST: &str = include_str!("../../../../data/srd/manifest.json");

static PARSED: [OnceLock<Vec<Value>>; 20] = [const { OnceLock::new() }; 20];

/// Every record in a file. The files are checked by `tools/srd/check.py` and a test parses
/// each one, so a parse failure here can only be a build of bad data.
pub(crate) fn records(file: &str) -> &'static [Value] {
    let i = FILES
        .iter()
        .position(|(name, _)| *name == file)
        .unwrap_or_else(|| panic!("{file} is not an embedded SRD file"));
    PARSED[i].get_or_init(|| {
        serde_json::from_str(FILES[i].1).unwrap_or_else(|e| panic!("{file} is invalid: {e}"))
    })
}

/// The file holding a category's records.
fn file_of(category: &str) -> Option<&'static str> {
    Some(match category {
        "armor" => "armor.json",
        "background" => "backgrounds.json",
        "class" => "classes.json",
        "condition" => "conditions.json",
        "feat" => "feats.json",
        "gear" => "gear.json",
        "glossary" => "glossary.json",
        "magic_item" => "magic_items.json",
        "mastery_property" => "mastery_properties.json",
        "monster" => "monsters.json",
        "mount" | "vehicle" => "mounts_vehicles.json",
        "rule" => "rules.json",
        "service" => "services.json",
        "species" => "species.json",
        "spell" => "spells.json",
        "subclass" => "subclasses.json",
        "tool" => "tools.json",
        "weapon_property" => "weapon_properties.json",
        "weapon" => "weapons.json",
        _ => return None,
    })
}

/// A record by id, including parts of records such as `species:dwarf#stonecunning`, which come
/// back as records of their own.
pub fn record(id: &str) -> Option<Value> {
    let (parent, part) = match id.split_once('#') {
        Some((parent, part)) => (parent, Some(part)),
        None => (id, None),
    };
    let file = file_of(parent.split(':').next()?)?;
    let found = records(file).iter().find(|r| r["id"] == parent)?;
    match part {
        None => Some(found.clone()),
        Some(part) => parts(found).into_iter().find(|p| {
            p["id"]
                .as_str()
                .is_some_and(|id| id.rsplit('#').next() == Some(part))
        }),
    }
}

/// The separately addable parts of a record: species traits, class features and options.
pub(crate) fn parts(parent: &Value) -> Vec<Value> {
    let id = parent["id"].as_str().unwrap_or_default();
    let name = parent["name"].as_str().unwrap_or_default();
    let list = |key: &str| parent[key].as_array().map_or(&[][..], Vec::as_slice);
    let part = |category: &str, part: &Value, suffix: &str, source: String| {
        let title = part["name"].as_str().unwrap_or_default();
        let markdown = part["markdown"].as_str().unwrap_or_default();
        json!({
            "id": format!("{id}#{}{suffix}", slug(title)),
            "name": title,
            "category": category,
            "page": parent["page"],
            "parent": id,
            "source": source,
            "level": part["level"],
            "summary": first_sentence(markdown),
            "markdown": markdown,
        })
    };
    let mut out = Vec::new();
    match parent["category"].as_str() {
        Some("species") => {
            for t in list("traits") {
                out.push(part("species_trait", t, "", name.to_owned()));
            }
        }
        Some(category @ ("class" | "subclass")) => {
            let kind = format!("{category}_feature");
            let mut seen: Vec<&str> = Vec::new();
            for f in list("features") {
                let title = f["name"].as_str().unwrap_or_default();
                let level = f["level"].as_u64().unwrap_or(1);
                // A feature named twice (Improved Brutal Strike) is told apart by its level.
                let suffix = if seen.contains(&title) {
                    format!("-{level}")
                } else {
                    String::new()
                };
                seen.push(title);
                out.push(part(&kind, f, &suffix, format!("{name} {level}")));
            }
            for group in list("option_groups") {
                for o in group["options"].as_array().map_or(&[][..], Vec::as_slice) {
                    out.push(part("class_option", o, "", name.to_owned()));
                }
            }
        }
        _ => {}
    }
    out
}

fn first_sentence(markdown: &str) -> String {
    let text = markdown
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    match text.find(". ") {
        Some(end) => text[..=end].to_owned(),
        None => text.to_owned(),
    }
}

/// The notice CC-BY-4.0 asks for, with what was changed.
pub fn attribution() -> String {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("the SRD manifest is valid");
    let notice = manifest["attribution"].as_str().unwrap_or_default();
    format!("{notice} Reformatted as structured data.")
}
