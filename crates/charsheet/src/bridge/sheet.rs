//! One open character. QML reads `view` (character + derived values as JSON) and edits
//! through invokables. Every successful edit is saved straight away.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, view, READ, NOTIFY)]
        #[qproperty(bool, loaded, READ, NOTIFY)]
        #[qproperty(QString, last_error, cxx_name = "lastError", READ, NOTIFY)]
        /// JSON array of recent rolls, newest first.
        #[qproperty(QString, rolls, READ, NOTIFY)]
        type CharacterSheet = super::CharacterSheetRust;

        #[qinvokable]
        fn load(self: Pin<&mut Self>, id: &QString) -> bool;

        #[qinvokable]
        fn close(self: Pin<&mut Self>);

        /// Static choice lists (classes, armor, abilities, conditions) as JSON.
        #[qinvokable]
        fn catalog(self: &Self) -> QString;

        /// Sets the JSON value at a JSON Pointer such as `/hp/current`.
        #[qinvokable]
        fn set(self: Pin<&mut Self>, pointer: &QString, value_json: &QString) -> bool;

        #[qinvokable]
        fn append(self: Pin<&mut Self>, pointer: &QString, item_json: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "removeAt"]
        fn remove_at(self: Pin<&mut Self>, pointer: &QString, index: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "moveItem"]
        fn move_item(self: Pin<&mut Self>, pointer: &QString, from: i32, to: i32) -> bool;

        #[qinvokable]
        fn damage(self: Pin<&mut Self>, amount: i32);

        #[qinvokable]
        fn heal(self: Pin<&mut Self>, amount: i32);

        #[qinvokable]
        #[cxx_name = "setTempHp"]
        fn set_temp_hp(self: Pin<&mut Self>, amount: i32);

        /// Spends a hit die and returns a short description of the result.
        #[qinvokable]
        #[cxx_name = "spendHitDie"]
        fn spend_hit_die(self: Pin<&mut Self>, class_index: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "shortRest"]
        fn short_rest(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "longRest"]
        fn long_rest(self: Pin<&mut Self>);

        /// Rolls an expression. `mode` is "normal", "advantage" or "disadvantage".
        /// Returns the roll as JSON, or "" if the expression is invalid (see lastError).
        #[qinvokable]
        fn roll(
            self: Pin<&mut Self>,
            expression: &QString,
            label: &QString,
            mode: &QString,
        ) -> QString;

        #[qinvokable]
        #[cxx_name = "clearRolls"]
        fn clear_rolls(self: Pin<&mut Self>);
    }
}

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use dnd_core::dice::{self, RollMode, RollResult};
use dnd_core::view::{SheetView, catalog};
use dnd_core::{Character, Library, edit};
use serde::Serialize;
use std::pin::Pin;

const MAX_ROLLS: usize = 50;

#[derive(Serialize)]
struct LoggedRoll {
    label: String,
    breakdown: String,
    #[serde(flatten)]
    result: RollResult,
}

pub struct CharacterSheetRust {
    view: QString,
    loaded: bool,
    last_error: QString,
    rolls: QString,
    character: Option<Character>,
    library: Option<Library>,
    roll_log: Vec<LoggedRoll>,
}

impl Default for CharacterSheetRust {
    fn default() -> Self {
        CharacterSheetRust {
            view: QString::from("{}"),
            loaded: false,
            last_error: QString::default(),
            rolls: QString::from("[]"),
            character: None,
            library: Library::open_default().ok(),
            roll_log: Vec::new(),
        }
    }
}

impl qobject::CharacterSheet {
    fn set_error(mut self: Pin<&mut Self>, message: &str) {
        let message = QString::from(message);
        if self.rust().last_error != message {
            self.as_mut().rust_mut().get_mut().last_error = message;
            self.last_error_changed();
        }
    }

    /// Re-serializes the view after a change and saves the character.
    fn publish(mut self: Pin<&mut Self>) {
        let (json, save_error) = {
            let this = self.rust();
            let Some(character) = &this.character else {
                return;
            };
            let json =
                serde_json::to_string(&SheetView::new(character)).unwrap_or_else(|_| "{}".into());
            let save_error = match &this.library {
                Some(lib) => lib
                    .save(character)
                    .err()
                    .map(|e| format!("Could not save: {e}")),
                None => Some("Could not save: no character library".to_owned()),
            };
            (json, save_error)
        };
        self.as_mut().rust_mut().get_mut().view = QString::from(&json);
        self.as_mut().view_changed();
        self.set_error(save_error.as_deref().unwrap_or(""));
    }

    /// Applies a change to the open character, then publishes and saves it.
    fn mutate<T>(
        mut self: Pin<&mut Self>,
        f: impl FnOnce(&mut Character) -> Result<T, String>,
    ) -> Option<T> {
        let result = match self.as_mut().rust_mut().get_mut().character.as_mut() {
            Some(c) => f(c),
            None => Err("No character is open".to_owned()),
        };
        match result {
            Ok(value) => {
                self.publish();
                Some(value)
            }
            Err(e) => {
                self.set_error(&e);
                None
            }
        }
    }

    pub fn load(mut self: Pin<&mut Self>, id: &QString) -> bool {
        let loaded = match (&self.rust().library, uuid::Uuid::parse_str(&id.to_string())) {
            (Some(lib), Ok(id)) => lib.load(id).map_err(|e| e.to_string()),
            (None, _) => Err("No character library".to_owned()),
            (_, Err(e)) => Err(format!("Invalid character id: {e}")),
        };
        match loaded {
            Ok(character) => {
                let this = self.as_mut().rust_mut().get_mut();
                this.character = Some(character);
                this.roll_log.clear();
                this.rolls = QString::from("[]");
                self.as_mut().rolls_changed();
                if !self.rust().loaded {
                    self.as_mut().rust_mut().get_mut().loaded = true;
                    self.as_mut().loaded_changed();
                }
                self.publish();
                true
            }
            Err(e) => {
                self.set_error(&e);
                false
            }
        }
    }

    pub fn close(mut self: Pin<&mut Self>) {
        let this = self.as_mut().rust_mut().get_mut();
        this.character = None;
        this.view = QString::from("{}");
        this.loaded = false;
        self.as_mut().view_changed();
        self.as_mut().loaded_changed();
    }

    pub fn catalog(&self) -> QString {
        QString::from(&serde_json::to_string(&catalog()).unwrap_or_default())
    }

    pub fn set(self: Pin<&mut Self>, pointer: &QString, value_json: &QString) -> bool {
        let pointer = pointer.to_string();
        let value = serde_json::from_str(&value_json.to_string());
        self.mutate(|c| {
            edit::set(c, &pointer, value.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
        })
        .is_some()
    }

    pub fn append(self: Pin<&mut Self>, pointer: &QString, item_json: &QString) -> bool {
        let pointer = pointer.to_string();
        let item = serde_json::from_str(&item_json.to_string());
        self.mutate(|c| {
            edit::push(c, &pointer, item.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
        })
        .is_some()
    }

    pub fn remove_at(self: Pin<&mut Self>, pointer: &QString, index: i32) -> bool {
        let pointer = pointer.to_string();
        let index = usize::try_from(index).map_err(|e| e.to_string());
        self.mutate(|c| edit::remove(c, &pointer, index?).map_err(|e| e.to_string()))
            .is_some()
    }

    pub fn move_item(self: Pin<&mut Self>, pointer: &QString, from: i32, to: i32) -> bool {
        let pointer = pointer.to_string();
        let indices = usize::try_from(from)
            .and_then(|f| Ok((f, usize::try_from(to)?)))
            .map_err(|e| e.to_string());
        self.mutate(|c| {
            let (from, to) = indices?;
            edit::move_item(c, &pointer, from, to).map_err(|e| e.to_string())
        })
        .is_some()
    }

    pub fn damage(self: Pin<&mut Self>, amount: i32) {
        self.mutate(|c| {
            c.take_damage(amount);
            Ok(())
        });
    }

    pub fn heal(self: Pin<&mut Self>, amount: i32) {
        self.mutate(|c| {
            c.heal(amount);
            Ok(())
        });
    }

    pub fn set_temp_hp(self: Pin<&mut Self>, amount: i32) {
        self.mutate(|c| {
            // Setting temp HP explicitly from the sheet replaces the value.
            c.hp.temp = amount.max(0);
            Ok(())
        });
    }

    pub fn spend_hit_die(self: Pin<&mut Self>, class_index: i32) -> QString {
        let message = self.mutate(|c| {
            let index = usize::try_from(class_index).map_err(|e| e.to_string())?;
            match c.spend_hit_die(index, &mut rand::rng()) {
                Some((roll, healed)) => Ok(format!("Rolled {roll}, regained {healed} HP")),
                None => Err("No hit dice of that type left".to_owned()),
            }
        });
        QString::from(&message.unwrap_or_default())
    }

    pub fn short_rest(self: Pin<&mut Self>) {
        self.mutate(|c| {
            c.short_rest();
            Ok(())
        });
    }

    pub fn long_rest(self: Pin<&mut Self>) {
        self.mutate(|c| {
            c.long_rest();
            Ok(())
        });
    }

    pub fn roll(
        mut self: Pin<&mut Self>,
        expression: &QString,
        label: &QString,
        mode: &QString,
    ) -> QString {
        let mode = match mode.to_string().as_str() {
            "advantage" => RollMode::Advantage,
            "disadvantage" => RollMode::Disadvantage,
            _ => RollMode::Normal,
        };
        match dice::roll(&expression.to_string(), mode) {
            Ok(result) => {
                let logged = LoggedRoll {
                    label: label.to_string(),
                    breakdown: result.breakdown(),
                    result,
                };
                let json = serde_json::to_string(&logged).unwrap_or_default();
                let this = self.as_mut().rust_mut().get_mut();
                this.roll_log.insert(0, logged);
                this.roll_log.truncate(MAX_ROLLS);
                this.rolls =
                    QString::from(&serde_json::to_string(&this.roll_log).unwrap_or_default());
                self.as_mut().rolls_changed();
                self.set_error("");
                QString::from(&json)
            }
            Err(e) => {
                self.set_error(&e.to_string());
                QString::default()
            }
        }
    }

    pub fn clear_rolls(mut self: Pin<&mut Self>) {
        let this = self.as_mut().rust_mut().get_mut();
        this.roll_log.clear();
        this.rolls = QString::from("[]");
        self.rolls_changed();
    }
}
