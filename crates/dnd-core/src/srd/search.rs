//! Ranked search over the index, plus the parts of records the index doesn't list.

use super::convert::options;
use super::data::{parts, record, records};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Gear entries that repeat a magic item, which is the one to add.
const HIDDEN: [&str; 2] = ["gear:potion-of-healing", "gear:spell-scroll"];

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Hit {
    pub id: String,
    pub name: String,
    pub category: String,
    pub summary: String,
    /// Short facts for filtering by eye, e.g. `level 3`, `evocation`.
    pub facets: Vec<String>,
    /// What has to be chosen before adding it: `base`, `variant` or `choice`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<String>,
    /// Filled in by a query such as `+1 longsword`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchResults {
    /// How many records matched, of which `results` holds the best.
    pub total: usize,
    pub results: Vec<Hit>,
}

#[derive(Debug, Deserialize)]
struct IndexRow {
    id: String,
    name: String,
    category: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    summary: String,
}

/// A row with its text folded for matching.
struct Row {
    hit: Hit,
    names: Vec<String>,
    keywords: String,
    summary: String,
}

impl Row {
    fn new(hit: Hit, aliases: &[String]) -> Row {
        let names = std::iter::once(&hit.name)
            .chain(aliases)
            .map(|n| fold(n))
            .collect();
        let keywords = fold(&hit.facets.join(" "));
        let summary = fold(&hit.summary);
        Row {
            hit,
            names,
            keywords,
            summary,
        }
    }
}

/// Folds text for matching: lowercase, no apostrophes, words split by single spaces.
fn fold(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().filter(|c| !matches!(c, '\'' | '’')) {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out.truncate(out.trim_end().len());
    out
}

fn rows() -> &'static [Row] {
    static ROWS: OnceLock<Vec<Row>> = OnceLock::new();
    ROWS.get_or_init(|| {
        let mut rows: Vec<Row> = records("index.json")
            .iter()
            .filter_map(|v| serde_json::from_value::<IndexRow>(v.clone()).ok())
            .filter(|r| !HIDDEN.contains(&r.id.as_str()))
            .map(|r| {
                let hit = Hit {
                    id: r.id,
                    name: r.name,
                    category: r.category,
                    summary: r.summary,
                    facets: r.keywords,
                    ..Default::default()
                };
                Row::new(hit, &r.aliases)
            })
            .collect();
        for file in ["species.json", "classes.json", "subclasses.json"] {
            for parent in records(file) {
                rows.extend(parts(parent).iter().map(part_row));
            }
        }
        rows
    })
}

fn part_row(part: &Value) -> Row {
    let text = |key: &str| part[key].as_str().unwrap_or_default().to_owned();
    let hit = Hit {
        id: text("id"),
        name: text("name"),
        category: text("category"),
        summary: text("summary"),
        facets: vec![text("source")],
        ..Default::default()
    };
    Row::new(hit, &[])
}

fn words_start_with(text: &str, tokens: &[&str]) -> bool {
    tokens
        .iter()
        .all(|t| text.split(' ').any(|word| word.starts_with(t)))
}

/// How well a row matches, best first; None if it doesn't.
fn rank(row: &Row, query: &str, tokens: &[&str]) -> Option<u8> {
    if row.names.iter().any(|n| n == query) {
        Some(0)
    } else if row.names.iter().any(|n| n.starts_with(query)) {
        Some(1)
    } else if row.names.iter().any(|n| words_start_with(n, tokens)) {
        Some(2)
    } else if row.names.iter().any(|n| n.contains(query)) {
        Some(3)
    } else if words_start_with(&format!("{} {}", row.names.join(" "), row.keywords), tokens) {
        Some(4)
    } else if words_start_with(&row.summary, tokens) {
        Some(5)
    } else {
        None
    }
}

/// Searches names, then keywords, then summaries. An empty query lists everything by name.
/// `categories` limits the search, and breaks ties in the order given.
pub fn search(raw: &str, categories: &[&str], limit: usize) -> SearchResults {
    let query = fold(raw);
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let order = |category: &str| {
        categories
            .iter()
            .position(|c| *c == category)
            .unwrap_or(usize::MAX)
    };
    let mut matches: Vec<(u8, usize, &Row)> = rows()
        .iter()
        .filter(|r| categories.is_empty() || order(&r.hit.category) != usize::MAX)
        .filter_map(|r| {
            let rank = if query.is_empty() {
                Some(0)
            } else {
                rank(r, &query, &tokens)
            };
            rank.map(|rank| (rank, order(&r.hit.category), r))
        })
        .collect();
    matches.sort_by(|a, b| {
        (a.0, a.1, a.2.hit.name.len(), &a.2.hit.name).cmp(&(
            b.0,
            b.1,
            b.2.hit.name.len(),
            &b.2.hit.name,
        ))
    });
    if query.is_empty() {
        matches.sort_by(|a, b| a.2.hit.name.cmp(&b.2.hit.name));
    }
    let mut seen = HashSet::new();
    matches.retain(|(_, _, r)| seen.insert(&r.hit.id));
    let shorthand = plus_item(raw, categories);
    SearchResults {
        total: matches.len() + usize::from(shorthand.is_some()),
        results: shorthand
            .into_iter()
            .chain(matches.into_iter().map(|(_, _, r)| r.hit.clone()))
            .take(limit)
            .map(|mut hit| {
                hit.needs = needs(&hit);
                hit
            })
            .collect(),
    }
}

/// `+1 longsword` as the magic item it means, with its base and variant filled in.
fn plus_item(query: &str, categories: &[&str]) -> Option<Hit> {
    let (bonus, name) = query.trim().split_once(' ')?;
    if !matches!(bonus, "+1" | "+2" | "+3")
        || !(categories.is_empty() || categories.contains(&"magic_item"))
    {
        return None;
    }
    let base = find(name, &["weapon", "armor"])?;
    let family = match base {
        "armor:shield" => "magic_item:shield-plus-1-plus-2-or-plus-3",
        id if id.starts_with("armor:") => "magic_item:armor-plus-1-plus-2-or-plus-3",
        _ => "magic_item:weapon-plus-1-plus-2-or-plus-3",
    };
    let row = rows().iter().find(|r| r.hit.id == family)?;
    let title = record(base)?["name"].as_str()?.to_owned();
    Some(Hit {
        name: format!("{bonus} {title}"),
        base: (base != "armor:shield").then(|| base.to_owned()),
        variant: Some(bonus.to_owned()),
        ..row.hit.clone()
    })
}

/// What's left to choose before a hit can be added.
fn needs(hit: &Hit) -> Vec<String> {
    let Ok(open) = options(&hit.id) else {
        return Vec::new();
    };
    let mut needs = Vec::new();
    if open.bases.len() > 1 && hit.base.is_none() {
        needs.push("base".to_owned());
    }
    if open.variants.len() > 1 && hit.variant.is_none() {
        needs.push("variant".to_owned());
    }
    if open.choice.is_some() {
        needs.push("choice".to_owned());
    }
    needs
}

/// The id of the record with exactly this name or alias, for looking up what a sheet entry is.
pub fn find(name: &str, categories: &[&str]) -> Option<&'static str> {
    let name = fold(name);
    if name.is_empty() {
        return None;
    }
    categories.iter().find_map(|category| {
        rows()
            .iter()
            .find(|r| r.hit.category == *category && r.names.contains(&name))
            .map(|r| r.hit.id.as_str())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first(query: &str, categories: &[&str]) -> String {
        search(query, categories, 5).results[0].id.clone()
    }

    #[test]
    fn every_row_resolves_to_a_record() {
        assert!(rows().len() > 1700 + 300);
        for r in rows() {
            let found = record(&r.hit.id).unwrap_or_else(|| panic!("{} is missing", r.hit.id));
            assert_eq!(found["name"], r.hit.name);
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut seen = HashSet::new();
        for r in rows() {
            assert!(seen.insert(&r.hit.id), "{} is repeated", r.hit.id);
        }
    }

    #[test]
    fn ranks_exact_names_first() {
        assert_eq!(first("fireball", &[]), "spell:fireball");
        assert_eq!(first("FIREBALL", &["spell"]), "spell:fireball");
        assert_eq!(first("shield", &["spell"]), "spell:shield");
        assert_eq!(first("shield", &["armor", "spell"]), "armor:shield");
    }

    #[test]
    fn ignores_apostrophes() {
        assert_eq!(first("hunters mark", &[]), "spell:hunters-mark");
        assert_eq!(first("hunter's mark", &[]), "spell:hunters-mark");
        assert_eq!(first("thieves tools", &["tool"]), "tool:thieves-tools");
    }

    #[test]
    fn finds_plurals_and_keywords() {
        assert_eq!(
            first("potion of healing", &["magic_item"]),
            "magic_item:potions-of-healing"
        );
        let wizard = search("level 3 wizard", &["spell"], 400);
        assert!(wizard.results.iter().any(|h| h.id == "spell:fireball"));
        assert!(!wizard.results.iter().any(|h| h.id == "spell:cure-wounds"));
    }

    #[test]
    fn finds_parts_of_records() {
        assert_eq!(
            first("stonecunning", &["species_trait"]),
            "species:dwarf#stonecunning"
        );
        assert_eq!(first("rage", &["class_feature"]), "class:barbarian#rage");
        assert_eq!(
            first("careful spell", &["class_option"]),
            "class:sorcerer#careful-spell"
        );
    }

    #[test]
    fn empty_query_lists_a_category() {
        let all = search("", &["condition"], 100);
        assert_eq!(all.total, 15);
        assert_eq!(all.results[0].name, "Blinded");
    }

    #[test]
    fn limit_keeps_the_total() {
        let some = search("", &["spell"], 3);
        assert_eq!(some.total, 339);
        assert_eq!(some.results.len(), 3);
    }

    #[test]
    fn find_is_exact() {
        assert_eq!(
            find("hunter's mark", &["spell"]),
            Some("spell:hunters-mark")
        );
        assert_eq!(find("hunter", &["spell"]), None);
        assert_eq!(find("", &["spell"]), None);
    }

    #[test]
    fn plus_shorthand_prefills_the_magic_item() {
        let hit = &search("+1 longsword", &[], 5).results[0];
        assert_eq!(hit.id, "magic_item:weapon-plus-1-plus-2-or-plus-3");
        assert_eq!(hit.name, "+1 Longsword");
        assert_eq!(hit.base.as_deref(), Some("weapon:longsword"));
        assert_eq!(hit.variant.as_deref(), Some("+1"));
        assert!(hit.needs.is_empty());

        let plate = &search("+3 plate armor", &["magic_item"], 5).results[0];
        assert_eq!(plate.id, "magic_item:armor-plus-1-plus-2-or-plus-3");
        let shield = &search("+2 shield", &[], 5).results[0];
        assert_eq!(shield.id, "magic_item:shield-plus-1-plus-2-or-plus-3");
        assert!(shield.needs.is_empty());
        assert!(search("+1 nonsense", &[], 5).results.is_empty());
    }

    #[test]
    fn hits_say_what_they_need() {
        let hit = &search("flame tongue", &["magic_item"], 1).results[0];
        assert_eq!(hit.needs, ["base"]);
        let hit = &search("ring of resistance", &["magic_item"], 1).results[0];
        assert_eq!(hit.needs, ["choice"]);
        let hit = &search("fireball", &["spell"], 1).results[0];
        assert!(hit.needs.is_empty());
    }

    #[test]
    fn hides_gear_that_repeats_a_magic_item() {
        assert!(!rows().iter().any(|r| HIDDEN.contains(&r.hit.id.as_str())));
    }

    #[test]
    fn attribution_names_the_licence_and_the_change() {
        let notice = super::super::attribution();
        assert!(notice.contains("SRD 5.2.1"));
        assert!(notice.contains("Creative Commons Attribution 4.0"));
        assert!(notice.contains("Reformatted"));
    }
}
