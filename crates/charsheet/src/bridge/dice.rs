//! The 3D dice table: throws a roll's dice and serves their meshes to DiceOverlay.qml.

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
        type DiceTable = super::DiceTableRust;

        /// Simulates throwing a roll's dice (a StoredRoll as JSON) on a table with the
        /// view's `aspect` (width / height). Returns the throw as JSON, or "" if the roll
        /// has no dice to throw.
        #[qinvokable]
        #[cxx_name = "throwRoll"]
        fn throw_roll(self: &Self, roll_json: &QString, aspect: f64) -> QString;

        /// A die's mesh and face labels as JSON, by name ("d4" … "d20", or "d100" for the
        /// tens die).
        #[qinvokable]
        fn mesh(self: &Self, shape: &QString) -> QString;
    }
}

use crate::dice3d::{self, Shape};
use cxx_qt_lib::QString;
use dnd_core::storage::StoredRoll;

#[derive(Default)]
pub struct DiceTableRust;

impl qobject::DiceTable {
    fn throw_roll(&self, roll_json: &QString, aspect: f64) -> QString {
        let Ok(roll) = serde_json::from_str::<StoredRoll>(&roll_json.to_string()) else {
            return QString::default();
        };
        let dice = dice3d::dice_of(&roll.result);
        if dice.is_empty() {
            return QString::default();
        }
        let thrown = dice3d::simulate(&dice, aspect as f32, rand::random());
        QString::from(&serde_json::to_string(&thrown).unwrap_or_default())
    }

    fn mesh(&self, shape: &QString) -> QString {
        Shape::from_name(&shape.to_string())
            .and_then(|s| serde_json::to_string(&dice3d::mesh(s)).ok())
            .map(|json| QString::from(&json))
            .unwrap_or_default()
    }
}
