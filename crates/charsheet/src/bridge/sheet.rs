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
use dnd_core::character::PlayError;
use dnd_core::dice::{self, RollMode, RollResult};
use dnd_core::effects::{Effect, Outcome, RollEffect};
use dnd_core::storage::{MAX_ROLLS, StoredRoll};
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

    /// Adds a roll to the log, saving it with the character.
    fn log_roll(mut self: Pin<&mut Self>, label: &str, result: RollResult) -> StoredRoll {
        let logged = StoredRoll::new(label, result);
        let save_error = match (&self.rust().library, &self.rust().character) {
            (Some(lib), Some(c)) => lib
                .add_roll(c.id, &logged)
                .err()
                .map(|e| format!("Could not save roll: {e}")),
            _ => None,
        };
        let this = self.as_mut().rust_mut().get_mut();
        this.roll_log.insert(0, logged.clone());
        this.roll_log.truncate(MAX_ROLLS);
        this.rolls = QString::from(&serde_json::to_string(&this.roll_log).unwrap_or_default());
        self.as_mut().rolls_changed();
        self.set_error(save_error.as_deref().unwrap_or(""));
        logged
    }

    /// Rolls an expression, logging it; sets the error and returns None if it's invalid.
    fn roll_logged(
        self: Pin<&mut Self>,
        expression: &str,
        label: &str,
        mode: RollMode,
    ) -> Option<StoredRoll> {
        match dice::roll(expression, mode) {
            Ok(result) => Some(self.log_roll(label, result)),
            Err(e) => {
                self.set_error(&format!("{label}: {e}"));
                None
            }
        }
    }

    /// Rolls an action's dice, applies healing or temp HP, and describes what happened.
    fn finish_outcome(mut self: Pin<&mut Self>, outcome: Option<Outcome>) -> QString {
        let Some(outcome) = outcome else {
            return QString::default();
        };
        let mut parts = vec![outcome.label.clone()];
        if !outcome.roll.trim().is_empty()
            && let Some(rolled) =
                self.as_mut()
                    .roll_logged(&outcome.roll, &outcome.label, RollMode::Normal)
        {
            let total = rolled.result.total;
            parts.push(match outcome.roll_effect {
                RollEffect::None => format!("rolled {total}"),
                RollEffect::Heal => format!("healed {total}"),
                RollEffect::TempHp => format!("{total} temp HP"),
            });
            if outcome.roll_effect != RollEffect::None {
                self.as_mut().mutate(|c| {
                    c.apply_roll(outcome.roll_effect, total);
                    Ok(())
                });
            }
        }
        if !outcome.spent.is_empty() {
            parts.push(outcome.spent);
        }
        QString::from(&parts.join(" · "))
    }

    pub fn load(mut self: Pin<&mut Self>, id: &QString) -> bool {
        let loaded = match (&self.rust().library, uuid::Uuid::parse_str(&id.to_string())) {
            (Some(lib), Ok(id)) => lib
                .load(id)
                .and_then(|c| Ok((c, lib.recent_rolls(id)?)))
                .map_err(|e| e.to_string()),
            (None, _) => Err("No character library".to_owned()),
            (_, Err(e)) => Err(format!("Invalid character id: {e}")),
        };
        match loaded {
            Ok((character, rolls)) => {
                let this = self.as_mut().rust_mut().get_mut();
                this.character = Some(character);
                this.rolls = QString::from(&serde_json::to_string(&rolls).unwrap_or_default());
                this.roll_log = rolls;
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
        match dice::roll(&expression.to_string(), roll_mode(mode)) {
            Ok(result) => {
                let logged = self.as_mut().log_roll(&label.to_string(), result);
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
        self.as_mut().set_error(save_error.as_deref().unwrap_or(""));
        let this = self.as_mut().rust_mut().get_mut();
        this.roll_log.clear();
        this.rolls = QString::from("[]");
        self.rolls_changed();
    }
}

fn index(i: i32) -> Result<usize, String> {
    usize::try_from(i).map_err(|e| e.to_string())
}

fn play(e: PlayError) -> String {
    e.to_string()
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
        let outcome = self.as_mut().mutate(|c| {
            c.use_item_action(index(item)?, index(action)?)
                .map_err(play)
        });
        self.finish_outcome(outcome)
    }

    pub fn use_feature_action(mut self: Pin<&mut Self>, feature: i32, action: i32) -> QString {
        let outcome = self.as_mut().mutate(|c| {
            c.use_feature_action(index(feature)?, index(action)?)
                .map_err(play)
        });
        self.finish_outcome(outcome)
    }

    pub fn cast_spell(mut self: Pin<&mut Self>, spell: i32, slot_level: i32) -> QString {
        let outcome = self.as_mut().mutate(|c| {
            let level = u8::try_from(slot_level).map_err(|e| e.to_string())?;
            c.cast_spell(index(spell)?, level).map_err(play)
        });
        self.finish_outcome(outcome)
    }

    pub fn weapon_attack(mut self: Pin<&mut Self>, attack: i32, mode: &QString) -> QString {
        let fired = self.as_mut().mutate(|c| {
            let a = c
                .weapon_attacks()
                .into_iter()
                .nth(index(attack)?)
                .ok_or("No such attack")?;
            if a.ranged && !a.thrown {
                c.fire(a.item).map_err(play)?;
            }
            Ok(a)
        });
        let Some(a) = fired else {
            return QString::default();
        };
        let modifier = match a.to_hit {
            0 => String::new(),
            n if n > 0 => format!("+{n}"),
            n => n.to_string(),
        };
        let label = format!("{} attack", a.name);
        let rolled = self.roll_logged(&format!("1d20{modifier}"), &label, roll_mode(mode));
        roll_json(rolled)
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
            let a = c
                .weapon_attacks()
                .into_iter()
                .nth(index(attack)?)
                .ok_or("No such attack")?;
            let expression = c.weapon_damage(&a, crit, &excluded).map_err(play)?;
            Ok((a.name, expression))
        });
        let Some((name, expression)) = rolled else {
            return QString::default();
        };
        let label = if crit {
            format!("{name} critical damage")
        } else {
            format!("{name} damage")
        };
        roll_json(self.roll_logged(&expression, &label, RollMode::Normal))
    }

    pub fn reload(self: Pin<&mut Self>, item: i32) {
        self.mutate(|c| c.reload(index(item)?).map_err(play));
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
        self.mutate(|c| c.end_effect(index(i)?).map(|_| ()).map_err(play));
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
