//! The System Reference Document 5.2.1, embedded in the binary: searching it, reading its
//! rules text and turning its records into sheet entries.
//!
//! Records stay as JSON values, as documented in `data/srd/README.md`. Prose is authoritative,
//! so anything that isn't a structured field comes from the hand-checked `mechanics.json` and
//! never from reading the text.

mod data;
mod search;

pub use data::{attribution, record};
pub use search::{Hit, SearchResults, find, search};

/// Short credit shown wherever SRD text is.
pub const SOURCE: &str = "SRD 5.2.1 (CC-BY-4.0)";

/// A stable slug for a name, as used in record ids.
pub(crate) fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().filter(|c| !matches!(c, '\'' | '’')) {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_owned()
}
