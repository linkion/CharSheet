//! One open character. QML reads `view` (character + derived values as JSON) and edits
//! through invokables. Every successful edit is saved straight away, after picking up any
//! changes made elsewhere (another window, or the MCP server) so they aren't overwritten.

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

        /// New rolls, oldest first, as a JSON array: those made here and those another
        /// process (such as the MCP server) added to the log. Rolls already in the log
        /// when the character opened aren't included.
        #[qsignal]
        #[cxx_name = "rollsThrown"]
        fn rolls_thrown(self: Pin<&mut Self>, rolls_json: QString);

        #[qinvokable]
        fn load(self: Pin<&mut Self>, id: &QString) -> bool;

        #[qinvokable]
        fn close(self: Pin<&mut Self>);

        /// Reloads the character if another window or process (such as the MCP server)
        /// changed it. Called when the MCP server signals a change, and on a timer.
        #[qinvokable]
        fn sync(self: Pin<&mut Self>);

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

        /// Uses an item's action: spends its cost, rolls, heals and applies its effect.
        /// Returns a short message for a notification.
        #[qinvokable]
        #[cxx_name = "useItemAction"]
        fn use_item_action(self: Pin<&mut Self>, item: i32, action: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "useFeatureAction"]
        fn use_feature_action(self: Pin<&mut Self>, feature: i32, action: i32) -> QString;

        /// Casts a spell with a slot of `slot_level`, or a free use when it's 0.
        #[qinvokable]
        #[cxx_name = "castSpell"]
        fn cast_spell(self: Pin<&mut Self>, spell: i32, slot_level: i32) -> QString;

        /// Attacks with a weapon (`attack` indexes `derived.weapon_attacks`), firing a shot
        /// if it's a ranged weapon. Returns the roll as JSON, or "" if it couldn't fire.
        #[qinvokable]
        #[cxx_name = "weaponAttack"]
        fn weapon_attack(self: Pin<&mut Self>, attack: i32, mode: &QString) -> QString;

        /// Rolls a weapon's damage with every rider except those named in `excluded_json`.
        /// Returns the roll as JSON, or "" on failure.
        #[qinvokable]
        #[cxx_name = "weaponDamage"]
        fn weapon_damage(
            self: Pin<&mut Self>,
            attack: i32,
            crit: bool,
            excluded_json: &QString,
        ) -> QString;

        #[qinvokable]
        fn reload(self: Pin<&mut Self>, item: i32);

        #[qinvokable]
        #[cxx_name = "applyEffect"]
        fn apply_effect(self: Pin<&mut Self>, effect_json: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "endEffect"]
        fn end_effect(self: Pin<&mut Self>, index: i32);

        #[qinvokable]
        #[cxx_name = "newTurn"]
        fn new_turn(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "endCombat"]
        fn end_combat(self: Pin<&mut Self>);
    }
}

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use dnd_core::dice::{self, RollMode};
use dnd_core::effects::Effect;
use dnd_core::play::{self, Played};
use dnd_core::storage::{MAX_ROLLS, StorageError, StoredRoll};
use dnd_core::view::{SheetView, catalog};
use dnd_core::{Character, Library, edit};
use std::pin::Pin;

pub struct CharacterSheetRust {
    view: QString,
    loaded: bool,
    last_error: QString,
    rolls: QString,
    character: Option<Character>,
    library: Option<Library>,
    roll_log: Vec<StoredRoll>,
    /// The newest roll announced by `rolls_thrown`, or already logged when the character opened.
    last_roll_id: i64,
    /// The open character's revision as of our last load or write.
    revision: i64,
    /// The library's data version as of our last check for outside changes.
    data_version: i64,
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
            last_roll_id: 0,
            revision: 0,
            data_version: 0,
        }
    }
}

impl CharacterSheetRust {
    /// The open character's current revision, after a write of our own.
    fn note_revision(&mut self) {
        if let (Some(lib), Some(c)) = (&self.library, &self.character)
            && let Ok(revision) = lib.revision(c.id)
        {
            self.revision = revision;
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

    /// Re-serializes the view after a change.
    fn refresh_view(mut self: Pin<&mut Self>) {
        let Some(character) = &self.rust().character else {
            return;
        };
        let json =
            serde_json::to_string(&SheetView::new(character)).unwrap_or_else(|_| "{}".into());
        self.as_mut().rust_mut().get_mut().view = QString::from(&json);
        self.view_changed();
    }

    /// Announces the rolls in the log newer than the last one announced.
    fn throw_new_rolls(mut self: Pin<&mut Self>) {
        let last = self.rust().last_roll_id;
        let fresh: Vec<&StoredRoll> = self
            .rust()
            .roll_log
            .iter()
            .rev()
            .filter(|r| r.id > last)
            .collect();
        let Some(newest) = fresh.last().map(|r| r.id) else {
            return;
        };
        let json = QString::from(&serde_json::to_string(&fresh).unwrap_or_default());
        self.as_mut().rust_mut().get_mut().last_roll_id = newest;
        self.rolls_thrown(json);
    }

    fn set_rolls(mut self: Pin<&mut Self>, rolls: Vec<StoredRoll>) {
        let this = self.as_mut().rust_mut().get_mut();
        this.rolls = QString::from(&serde_json::to_string(&rolls).unwrap_or_default());
        this.roll_log = rolls;
        self.rolls_changed();
    }

    /// Re-serializes the view after a change and saves the character.
    fn publish(mut self: Pin<&mut Self>) {
        self.as_mut().refresh_view();
        let this = self.as_mut().rust_mut().get_mut();
        let saved = match (&this.library, &this.character) {
            (Some(lib), Some(c)) => lib.save(c).map_err(|e| format!("Could not save: {e}")),
            (None, _) => Err("Could not save: no character library".to_owned()),
            (_, None) => return,
        };
        let save_error = match saved {
            Ok(revision) => {
                this.revision = revision;
                None
            }
            Err(e) => Some(e),
        };
        self.set_error(save_error.as_deref().unwrap_or(""));
    }

    /// Picks up changes another connection made to the open character since we last
    /// looked, so that edits never overwrite them.
    fn reload_if_changed(mut self: Pin<&mut Self>) {
        let this = self.as_mut().rust_mut().get_mut();
        let (Some(lib), Some(c)) = (&this.library, &this.character) else {
            return;
        };
        let id = c.id;
        let Ok(data_version) = lib.data_version() else {
            return;
        };
        if data_version == this.data_version {
            return;
        }
        this.data_version = data_version;
        let reloaded = match lib.revision(id) {
            Ok(revision) if revision == this.revision => return,
            Ok(revision) => lib
                .load(id)
                .and_then(|c| Ok((revision, c, lib.recent_rolls(id)?))),
            Err(e) => Err(e),
        };
        match reloaded {
            Ok((revision, character, rolls)) => {
                let this = self.as_mut().rust_mut().get_mut();
                this.revision = revision;
                this.character = Some(character);
                self.as_mut().set_rolls(rolls);
                self.as_mut().throw_new_rolls();
                self.refresh_view();
            }
            Err(StorageError::NotFound(_)) => {
                self.as_mut().close();
                self.set_error("This character was deleted elsewhere");
            }
            Err(e) => self.set_error(&format!("Could not reload: {e}")),
        }
    }

    /// Applies a change to the open character, then publishes and saves it.
    fn mutate<T>(
        mut self: Pin<&mut Self>,
        f: impl FnOnce(&mut Character) -> Result<T, String>,
    ) -> Option<T> {
        self.as_mut().reload_if_changed();
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

    /// Adds a roll to the log, saving it with the character.
    fn log_roll(mut self: Pin<&mut Self>, mut logged: StoredRoll) -> StoredRoll {
        let save_error = match (&self.rust().library, &self.rust().character) {
            (Some(lib), Some(c)) => match lib.add_roll(c.id, &logged) {
                Ok(id) => {
                    logged.id = id;
                    None
                }
                Err(e) => Some(format!("Could not save roll: {e}")),
            },
            _ => None,
        };
        let this = self.as_mut().rust_mut().get_mut();
        this.note_revision();
        let mut rolls = std::mem::take(&mut this.roll_log);
        rolls.insert(0, logged.clone());
        rolls.truncate(MAX_ROLLS);
        self.as_mut().set_rolls(rolls);
        self.as_mut().set_error(save_error.as_deref().unwrap_or(""));
        if logged.id == 0 {
            // Unsaved, so it has no id to announce it by.
            let json = serde_json::to_string(&[&logged]).unwrap_or_default();
            self.rolls_thrown(QString::from(&json));
        } else {
            self.throw_new_rolls();
        }
        logged
    }

    /// Logs what an action rolled and returns its message.
    fn finish(mut self: Pin<&mut Self>, played: Option<Played>) -> QString {
        let Some(played) = played else {
            return QString::default();
        };
        for roll in played.rolls {
            self.as_mut().log_roll(roll);
        }
        QString::from(&played.message)
    }

    pub fn load(mut self: Pin<&mut Self>, id: &QString) -> bool {
        let loaded = match (&self.rust().library, uuid::Uuid::parse_str(&id.to_string())) {
            (Some(lib), Ok(id)) => (|| {
                Ok((
                    lib.data_version()?,
                    lib.revision(id)?,
                    lib.load(id)?,
                    lib.recent_rolls(id)?,
                ))
            })()
            .map_err(|e: StorageError| e.to_string()),
            (None, _) => Err("No character library".to_owned()),
            (_, Err(e)) => Err(format!("Invalid character id: {e}")),
        };
        match loaded {
            Ok((data_version, revision, character, rolls)) => {
                let this = self.as_mut().rust_mut().get_mut();
                this.character = Some(character);
                this.data_version = data_version;
                this.revision = revision;
                this.last_roll_id = rolls.first().map_or(0, |r| r.id);
                self.as_mut().set_rolls(rolls);
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

    pub fn sync(self: Pin<&mut Self>) {
        self.reload_if_changed();
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
        let message =
            self.mutate(|c| play::spend_hit_die(c, index(class_index)?).map_err(|e| e.to_string()));
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
        self.as_mut().reload_if_changed();
        match dice::roll(&expression.to_string(), roll_mode(mode)) {
            Ok(result) => {
                let logged = self
                    .as_mut()
                    .log_roll(StoredRoll::new(label.to_string(), result));
                QString::from(&serde_json::to_string(&logged).unwrap_or_default())
            }
            Err(e) => {
                self.set_error(&e.to_string());
                QString::default()
            }
        }
    }

    pub fn clear_rolls(mut self: Pin<&mut Self>) {
        let save_error = match (&self.rust().library, &self.rust().character) {
            (Some(lib), Some(c)) => lib
                .clear_rolls(c.id)
                .err()
                .map(|e| format!("Could not clear rolls: {e}")),
            _ => None,
        };
        self.as_mut().rust_mut().get_mut().note_revision();
        self.as_mut().set_error(save_error.as_deref().unwrap_or(""));
        self.set_rolls(Vec::new());
    }
}

fn index(i: i32) -> Result<usize, String> {
    usize::try_from(i).map_err(|e| e.to_string())
}

fn roll_json(roll: Option<StoredRoll>) -> QString {
    roll.and_then(|r| serde_json::to_string(&r).ok())
        .map(|json| QString::from(&json))
        .unwrap_or_default()
}

fn roll_mode(mode: &QString) -> RollMode {
    match mode.to_string().as_str() {
        "advantage" => RollMode::Advantage,
        "disadvantage" => RollMode::Disadvantage,
        _ => RollMode::Normal,
    }
}

impl qobject::CharacterSheet {
    pub fn use_item_action(mut self: Pin<&mut Self>, item: i32, action: i32) -> QString {
        let played = self.as_mut().mutate(|c| {
            play::use_item_action(c, index(item)?, index(action)?).map_err(|e| e.to_string())
        });
        self.finish(played)
    }

    pub fn use_feature_action(mut self: Pin<&mut Self>, feature: i32, action: i32) -> QString {
        let played = self.as_mut().mutate(|c| {
            play::use_feature_action(c, index(feature)?, index(action)?).map_err(|e| e.to_string())
        });
        self.finish(played)
    }

    pub fn cast_spell(mut self: Pin<&mut Self>, spell: i32, slot_level: i32) -> QString {
        let played = self.as_mut().mutate(|c| {
            let level = u8::try_from(slot_level).map_err(|e| e.to_string())?;
            play::cast_spell(c, index(spell)?, level).map_err(|e| e.to_string())
        });
        self.finish(played)
    }

    pub fn weapon_attack(mut self: Pin<&mut Self>, attack: i32, mode: &QString) -> QString {
        let mode = roll_mode(mode);
        let rolled = self
            .as_mut()
            .mutate(|c| play::weapon_attack(c, index(attack)?, mode).map_err(|e| e.to_string()));
        roll_json(rolled.map(|r| self.log_roll(r)))
    }

    pub fn weapon_damage(
        mut self: Pin<&mut Self>,
        attack: i32,
        crit: bool,
        excluded_json: &QString,
    ) -> QString {
        let excluded: Vec<String> =
            serde_json::from_str(&excluded_json.to_string()).unwrap_or_default();
        let rolled = self.as_mut().mutate(|c| {
            play::weapon_damage(c, index(attack)?, crit, &excluded).map_err(|e| e.to_string())
        });
        roll_json(rolled.map(|r| self.log_roll(r)))
    }

    pub fn reload(self: Pin<&mut Self>, item: i32) {
        self.mutate(|c| c.reload(index(item)?).map_err(|e| e.to_string()));
    }

    pub fn apply_effect(self: Pin<&mut Self>, effect_json: &QString) -> bool {
        let effect = serde_json::from_str::<Effect>(&effect_json.to_string());
        self.mutate(|c| {
            c.apply_effect(effect.map_err(|e| e.to_string())?);
            Ok(())
        })
        .is_some()
    }

    pub fn end_effect(self: Pin<&mut Self>, i: i32) {
        self.mutate(|c| {
            c.end_effect(index(i)?)
                .map(|_| ())
                .map_err(|e| e.to_string())
        });
    }

    pub fn new_turn(self: Pin<&mut Self>) {
        self.mutate(|c| {
            c.new_turn();
            Ok(())
        });
    }

    pub fn end_combat(self: Pin<&mut Self>) {
        self.mutate(|c| {
            c.end_combat();
            Ok(())
        });
    }
}
