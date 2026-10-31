//! Links between records. The source text names other records in plain Markdown; turning
//! those names into `srd:` links lets a reader follow them in the rules lookup.

use super::data::records;
use super::find;

/// Links italicised spell and item names, and condition names followed by "condition".
pub fn linkify(markdown: &str) -> String {
    link_conditions(&link_italics(markdown))
}

fn link_italics(markdown: &str) -> String {
    let mut out = String::new();
    let mut rest = markdown;
    while let Some(start) = rest.find('*') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        // Runs of asterisks are bold or bold italic, which hold headings rather than names.
        let run = rest.len() - rest.trim_start_matches('*').len();
        if run > 1 {
            out.push_str(&rest[..run]);
            rest = &rest[run..];
            continue;
        }
        let inner = &rest[1..];
        let Some(end) = inner.find(['*', '\n']) else {
            out.push('*');
            rest = inner;
            continue;
        };
        let name = &inner[..end];
        let closed = inner[end..].starts_with('*');
        match find(name, &["spell", "magic_item"]).filter(|_| closed) {
            Some(id) => {
                out.push_str(&format!("[*{name}*](srd:{id})"));
                rest = &inner[end + 1..];
            }
            None => {
                out.push('*');
                rest = inner;
            }
        }
    }
    out.push_str(rest);
    out
}

fn link_conditions(markdown: &str) -> String {
    let mut text = markdown.to_owned();
    for condition in records("conditions.json") {
        let (Some(name), Some(id)) = (condition["name"].as_str(), condition["id"].as_str()) else {
            continue;
        };
        let linked = format!("[{name}](srd:{id}) condition");
        text = text.replace(&format!("{name} condition"), &linked);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_spell_names() {
        assert_eq!(
            linkify("the effect of the *Bless* spell"),
            "the effect of the [*Bless*](srd:spell:bless) spell"
        );
    }

    #[test]
    fn links_conditions() {
        assert_eq!(
            linkify("has the Prone condition."),
            "has the [Prone](srd:condition:prone) condition."
        );
    }

    #[test]
    fn leaves_everything_else_alone() {
        for text in [
            "***Blinded.*** You can't see.",
            "**Bless** and *not a spell* and a lone * star",
            "*Level 3 Evocation (Wizard)*\n\ntext",
            "*Open italic\nthen more",
        ] {
            assert_eq!(linkify(text), text);
        }
    }

    #[test]
    fn linked_text_still_renders_as_a_link_once() {
        let once = linkify("*Bless* and the Prone condition");
        assert_eq!(once.matches("srd:").count(), 2);
    }
}
