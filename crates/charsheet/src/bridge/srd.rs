//! The System Reference Document for QML: searching it, reading records and what a record
//! leaves open. Results are JSON strings, like the sheet's view.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type Srd = super::SrdRust;

        /// Search results as JSON. `categories_json` is a list of categories, or empty for all.
        #[qinvokable]
        fn search(self: &Self, query: &QString, categories_json: &QString, limit: i32) -> QString;

        /// A record as JSON with its links filled in, or "" if there's no such record. `about`
        /// is the licence notice.
        #[qinvokable]
        fn record(self: &Self, id: &QString) -> QString;

        /// The id of the record with exactly this name, or "".
        #[qinvokable]
        fn find(self: &Self, name: &QString, categories_json: &QString) -> QString;

        /// What's left to choose for a record, as JSON.
        #[qinvokable]
        fn options(self: &Self, id: &QString) -> QString;
    }
}

use cxx_qt_lib::QString;
use dnd_core::srd;
use serde_json::{Value, json};

#[derive(Default)]
pub struct SrdRust;

fn categories(json: &QString) -> Vec<String> {
    serde_json::from_str(&json.to_string()).unwrap_or_default()
}

fn borrowed(list: &[String]) -> Vec<&str> {
    list.iter().map(String::as_str).collect()
}

impl qobject::Srd {
    fn search(&self, query: &QString, categories_json: &QString, limit: i32) -> QString {
        let list = categories(categories_json);
        let limit = usize::try_from(limit).unwrap_or(0);
        let results = srd::search(&query.to_string(), &borrowed(&list), limit);
        QString::from(&json!(results).to_string())
    }

    fn record(&self, id: &QString) -> QString {
        let id = id.to_string();
        if id == "about" {
            let notice = json!({
                "id": "about",
                "name": "About this text",
                "markdown": srd::attribution(),
            });
            return QString::from(&notice.to_string());
        }
        let Some(record) = srd::record(&id) else {
            return QString::default();
        };
        let text = record["markdown"].as_str().unwrap_or_default();
        let shown = json!({
            "id": record["id"],
            "name": record["name"],
            "category": record["category"],
            "page": record["page"],
            "markdown": srd::linkify(text),
        });
        QString::from(&shown.to_string())
    }

    fn find(&self, name: &QString, categories_json: &QString) -> QString {
        let list = categories(categories_json);
        QString::from(srd::find(&name.to_string(), &borrowed(&list)).unwrap_or_default())
    }

    fn options(&self, id: &QString) -> QString {
        let open = srd::options(&id.to_string()).map_or(Value::Null, |o| json!(o));
        QString::from(&open.to_string())
    }
}
