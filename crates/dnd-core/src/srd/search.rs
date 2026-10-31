//! Ranked search over the index, plus the parts of records the index doesn't list.

use super::data::{parts, records};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Gear entries that repeat a magic item, which is the one to add.
const HIDDEN: [&str; 2] = ["gear:potion-of-healing", "gear:spell-scroll"];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hit {
    pub id: String,
    pub name: String,
    pub category: String,
    pub summary: String,
    /// Short facts for filtering by eye, e.g. `level 3`, `evocation`.
    pub facets: Vec<String>,
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
pub fn search(query: &str, categories: &[&str], limit: usize) -> SearchResults {
    let query = fold(query);
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
    SearchResults {
        total: matches.len(),
        results: matches
            .into_iter()
            .take(limit)
            .map(|(_, _, r)| r.hit.clone())
            .collect(),
    }
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
    use super::super::data::record;
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
