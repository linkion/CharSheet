//! The MCP tools. Every call loads the character fresh from the library and saves it
//! straight back, so the server never holds stale data and the app (which watches the
//! library) shows the change within a second.

use dnd_core::abilities::{Proficiency, Skill};
use dnd_core::character::PlayError;
use dnd_core::dice::{self, RollMode};
use dnd_core::effects::Effect;
use dnd_core::play::{self, Played};
use dnd_core::storage::{StoredRoll, Summary};
use dnd_core::view::{SheetView, catalog};
use dnd_core::{Character, Library, edit};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const INSTRUCTIONS: &str = "\
Reads, edits and plays D&D 5e (2024) characters stored by the CharSheet app. Changes are \
saved immediately and show up in the app if the character is open there.

Characters are named by id or by name (case-insensitive). Start with list_characters, then \
get_character, which returns {character, derived}: `character` is the stored sheet and \
`derived` holds computed values (AC, modifiers, saves, skills, spell slots, weapon_attacks, \
actions) that can't be edited directly. The whole sheet is large; pass a pointer such as \
/inventory or /derived/weapon_attacks to read just one part.

Edit any part of `character` with JSON Pointers into it: /name, /hp/current, \
/base_scores/0 (STR, DEX, CON, INT, WIS, CHA), /skills/stealth, /classes/0/level, \
/inventory/2/quantity, /features/1/modifiers/0/value. append_item takes partial objects and \
fills in defaults. Enum values are lowercase snake_case; get_catalog lists every valid choice. \
Invalid edits are rejected and change nothing; apply_edits makes several edits at once, all \
or nothing.

For play, prefer the play tools to raw edits, since they follow the rules (temp HP absorbs \
damage first, rests recharge uses, casting spends the right slot) and log their dice rolls: \
indices are positions in character.inventory, character.features, character.spells, \
character.classes, character.effects, and derived.weapon_attacks.";

#[derive(Deserialize, JsonSchema)]
pub struct CharacterArg {
    /// Character id or name (case-insensitive).
    pub character: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct NameArg {
    /// Name of the new character.
    pub name: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ImportArg {
    /// Path of a character JSON file, as written by export_character or the app.
    pub path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ExportArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// Where to write the JSON file. An existing file is replaced.
    pub path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// A JSON Pointer into the character, e.g. /inventory or /hp, or into the computed
    /// values under /derived, e.g. /derived/weapon_attacks. Omit for the whole sheet.
    pub pointer: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SetArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// JSON Pointer of the field, e.g. /hp/max or /attacks/0/name.
    pub pointer: String,
    /// The new value. Use null to clear an optional field such as /armor.
    pub value: Value,
}

#[derive(Deserialize, JsonSchema)]
pub struct AppendArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// JSON Pointer of a list, e.g. /inventory, /features or /inventory/3/actions.
    pub pointer: String,
    /// The new entry. Missing fields get defaults, so {"name": "Rope"} is enough.
    #[serde(default)]
    pub item: Option<Value>,
}

#[derive(Deserialize, JsonSchema)]
pub struct RemoveArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// JSON Pointer of a list, e.g. /spells.
    pub pointer: String,
    /// Position of the entry to remove, from 0.
    pub index: usize,
}

#[derive(Deserialize, JsonSchema)]
pub struct MoveArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// JSON Pointer of a list, e.g. /attacks.
    pub pointer: String,
    pub from: usize,
    pub to: usize,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EditOp {
    Set {
        pointer: String,
        value: Value,
    },
    Append {
        pointer: String,
        #[serde(default)]
        item: Option<Value>,
    },
    Remove {
        pointer: String,
        index: usize,
    },
    Move {
        pointer: String,
        from: usize,
        to: usize,
    },
}

#[derive(Deserialize, JsonSchema)]
pub struct EditsArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// Edits applied in order, e.g. [{"op": "set", "pointer": "/hp/max", "value": 45},
    /// {"op": "append", "pointer": "/features", "item": {"name": "Alert"}}].
    pub edits: Vec<EditOp>,
}

#[derive(Deserialize, JsonSchema)]
pub struct AmountArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    pub amount: i32,
}

#[derive(Deserialize, JsonSchema)]
pub struct HitDieArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// Which class's hit die to spend: its position in character.classes.
    #[serde(default)]
    pub class_index: usize,
}

#[derive(Deserialize, JsonSchema, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Normal,
    Advantage,
    Disadvantage,
}

impl From<Mode> for RollMode {
    fn from(mode: Mode) -> RollMode {
        match mode {
            Mode::Normal => RollMode::Normal,
            Mode::Advantage => RollMode::Advantage,
            Mode::Disadvantage => RollMode::Disadvantage,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct RollArg {
    /// Character id or name (case-insensitive); the roll goes in their roll log.
    pub character: String,
    /// Dice expression such as 1d20+5, 2d6+1d4-1 or 4d6kh3.
    pub expression: String,
    /// What the roll is for, shown in the log. Defaults to the expression.
    pub label: Option<String>,
    /// Advantage or disadvantage, for d20 rolls.
    #[serde(default)]
    pub mode: Mode,
}

#[derive(Deserialize, JsonSchema)]
pub struct ActionArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// Position of the item in character.inventory, or the feature in character.features.
    pub index: usize,
    /// Position of the action in that entry's actions list.
    #[serde(default)]
    pub action: usize,
}

#[derive(Deserialize, JsonSchema)]
pub struct CastArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// Position of the spell in character.spells.
    pub spell: usize,
    /// Level of the spell slot to spend, or 0 for one of the spell's free casts. Cantrips
    /// cost nothing either way.
    pub slot_level: u8,
}

#[derive(Deserialize, JsonSchema)]
pub struct AttackArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// Position of the attack in derived.weapon_attacks.
    pub attack: usize,
    #[serde(default)]
    pub mode: Mode,
}

#[derive(Deserialize, JsonSchema)]
pub struct WeaponDamageArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// Position of the attack in derived.weapon_attacks.
    pub attack: usize,
    /// Doubles the dice for a critical hit.
    #[serde(default)]
    pub crit: bool,
    /// Names of riders (from the attack's riders) to leave out of this hit.
    #[serde(default)]
    pub excluded_riders: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct IndexArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// Position in the relevant list (character.inventory for reload, character.effects for
    /// end_effect).
    pub index: usize,
}

#[derive(Deserialize, JsonSchema)]
pub struct EffectArg {
    /// Character id or name (case-insensitive).
    pub character: String,
    /// The effect, e.g. {"name": "Bless", "concentration": true, "rounds": 10,
    /// "modifiers": [{"kind": "note", "text": "+1d4 to attacks and saves"}]}.
    pub effect: Value,
}

#[derive(Clone)]
pub struct CharSheetServer {
    library: Arc<Mutex<Library>>,
    tool_router: ToolRouter<Self>,
}

type ToolResult = Result<String, String>;

fn to_json(value: &impl serde::Serialize) -> ToolResult {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

fn err(e: impl ToString) -> String {
    e.to_string()
}

/// Finds a character by id or case-insensitive name.
fn find(lib: &Library, character: &str) -> Result<Uuid, String> {
    if let Ok(id) = Uuid::parse_str(character.trim()) {
        return Ok(id);
    }
    let wanted = character.trim().to_lowercase();
    let matches: Vec<Summary> = lib
        .list()
        .map_err(err)?
        .into_iter()
        .filter(|s| s.name.to_lowercase() == wanted)
        .collect();
    match matches.as_slice() {
        [one] => Ok(one.id),
        [] => Err(format!(
            "No character named \"{character}\"; list_characters shows them all"
        )),
        many => Err(format!(
            "Several characters are named \"{character}\"; use an id: {}",
            many.iter()
                .map(|s| format!("{} ({})", s.id, s.class_summary))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The value at `pointer` in the character, after an edit.
fn value_at(c: &Character, pointer: &str) -> ToolResult {
    let value = serde_json::to_value(c).map_err(err)?;
    let found = value
        .pointer(pointer)
        .ok_or_else(|| format!("no field at {pointer}"))?;
    to_json(&json!({ "pointer": pointer, "value": found }))
}

/// The state that play changes, returned after every play tool so the agent can see the
/// result without fetching the whole sheet.
fn status(c: &Character) -> Value {
    let (pact_max, pact_level) = c.pact_slots();
    let slots: Vec<Value> = c
        .spell_slots_max()
        .iter()
        .zip(c.spell_slots_used)
        .enumerate()
        .filter(|(_, (max, _))| **max > 0)
        .map(|(i, (max, used))| json!({ "level": i + 1, "left": max - used, "max": max }))
        .collect();
    json!({
        "name": c.name,
        "hp": { "current": c.hp.current, "max": c.max_hp(), "temp": c.hp.temp },
        "death_saves": c.death_saves,
        "conditions": c.conditions,
        "exhaustion": c.exhaustion,
        "round": c.round,
        "concentrating": c.concentrating_on().map(|e| &e.name),
        "effects": c.effects.iter().map(|e| &e.name).collect::<Vec<_>>(),
        "spell_slots": slots,
        "pact_slots": (pact_max > 0).then(|| json!({
            "level": pact_level,
            "left": pact_max - c.pact_slots_used.min(pact_max),
            "max": pact_max,
        })),
    })
}

fn roll_summary(r: &StoredRoll) -> Value {
    json!({
        "label": r.label,
        "total": r.result.total,
        "breakdown": r.breakdown,
        "natural": r.result.natural,
    })
}

fn played_json(c: &Character, played: &Played) -> ToolResult {
    to_json(&json!({
        "message": played.message,
        "rolls": played.rolls.iter().map(roll_summary).collect::<Vec<_>>(),
        "status": status(c),
    }))
}

fn play_err(e: PlayError) -> String {
    e.to_string()
}

impl CharSheetServer {
    pub fn new(library: Library) -> CharSheetServer {
        CharSheetServer {
            library: Arc::new(Mutex::new(library)),
            tool_router: Self::tool_router(),
        }
    }

    fn with_library<T>(&self, f: impl FnOnce(&Library) -> Result<T, String>) -> Result<T, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "the library is unavailable after an earlier failure".to_owned())?;
        f(&lib)
    }

    fn read<T>(
        &self,
        character: &str,
        f: impl FnOnce(&Library, &Character) -> Result<T, String>,
    ) -> Result<T, String> {
        self.with_library(|lib| {
            let c = lib.load(find(lib, character)?).map_err(err)?;
            f(lib, &c)
        })
    }

    /// Loads the character, applies `f` and saves it, then logs any rolls `f` made. Nothing
    /// is saved if `f` fails.
    fn update(
        &self,
        character: &str,
        f: impl FnOnce(&mut Character) -> Result<(String, Vec<StoredRoll>), String>,
    ) -> ToolResult {
        self.with_library(|lib| {
            let mut c = lib.load(find(lib, character)?).map_err(err)?;
            let (output, rolls) = f(&mut c)?;
            lib.save(&c).map_err(err)?;
            for roll in &rolls {
                lib.add_roll(c.id, roll).map_err(err)?;
            }
            Ok(output)
        })
    }

    /// An update that returns the play status afterwards.
    fn play(
        &self,
        character: &str,
        f: impl FnOnce(&mut Character) -> Result<(), String>,
    ) -> ToolResult {
        self.update(character, |c| {
            f(c)?;
            Ok((to_json(&json!({ "status": status(c) }))?, Vec::new()))
        })
    }

    fn played(
        &self,
        character: &str,
        f: impl FnOnce(&mut Character) -> Result<Played, PlayError>,
    ) -> ToolResult {
        self.update(character, |c| {
            let played = f(c).map_err(play_err)?;
            Ok((played_json(c, &played)?, played.rolls))
        })
    }

    fn rolled(
        &self,
        character: &str,
        f: impl FnOnce(&mut Character) -> Result<StoredRoll, PlayError>,
    ) -> ToolResult {
        self.update(character, |c| {
            let roll = f(c).map_err(play_err)?;
            let output = to_json(&json!({ "roll": roll_summary(&roll), "status": status(c) }))?;
            Ok((output, vec![roll]))
        })
    }
}

#[tool_router]
impl CharSheetServer {
    // --- Library ------------------------------------------------------------------------

    #[tool(
        description = "List every character: id, name, species, classes and level.",
        annotations(read_only_hint = true)
    )]
    async fn list_characters(&self) -> ToolResult {
        self.with_library(|lib| to_json(&lib.list().map_err(err)?))
    }

    #[tool(description = "Create a new level 1 character with default stats. Returns its summary.")]
    async fn create_character(&self, Parameters(a): Parameters<NameArg>) -> ToolResult {
        let name = match a.name.trim() {
            "" => "New Adventurer",
            name => name,
        };
        self.with_library(|lib| to_json(&Summary::from(&lib.create(name).map_err(err)?)))
    }

    #[tool(description = "Copy a character, named \"<name> (copy)\". Returns the copy's summary.")]
    async fn duplicate_character(&self, Parameters(a): Parameters<CharacterArg>) -> ToolResult {
        self.with_library(|lib| {
            let copy = lib.duplicate(find(lib, &a.character)?).map_err(err)?;
            to_json(&Summary::from(&copy))
        })
    }

    #[tool(
        description = "Permanently delete a character and its roll log.",
        annotations(destructive_hint = true)
    )]
    async fn delete_character(&self, Parameters(a): Parameters<CharacterArg>) -> ToolResult {
        self.with_library(|lib| {
            let id = find(lib, &a.character)?;
            let name = lib.load(id).map_err(err)?.name;
            lib.delete(id).map_err(err)?;
            Ok(format!("Deleted {name} ({id})"))
        })
    }

    #[tool(
        description = "Import a character from a JSON file into the library. Returns its summary."
    )]
    async fn import_character(&self, Parameters(a): Parameters<ImportArg>) -> ToolResult {
        self.with_library(|lib| {
            let c = lib.import(Path::new(&a.path)).map_err(err)?;
            to_json(&Summary::from(&c))
        })
    }

    #[tool(description = "Write a character to a JSON file, replacing any file at that path.")]
    async fn export_character(&self, Parameters(a): Parameters<ExportArg>) -> ToolResult {
        self.with_library(|lib| {
            let id = find(lib, &a.character)?;
            lib.export(id, Path::new(&a.path)).map_err(err)?;
            Ok(format!("Exported to {}", a.path))
        })
    }

    // --- Reading ------------------------------------------------------------------------

    #[tool(
        description = "Read a character. Without a pointer, returns {character, derived}: the \
            editable sheet and the computed values (large). With a pointer, returns just that \
            part: /inventory, /spells, /hp and so on from the character, or /derived/... for \
            computed values such as /derived/weapon_attacks or /derived/skills.",
        annotations(read_only_hint = true)
    )]
    async fn get_character(&self, Parameters(a): Parameters<GetArg>) -> ToolResult {
        self.read(&a.character, |_, c| match a.pointer.as_deref() {
            None | Some("") => to_json(&SheetView::new(c)),
            Some(pointer) if pointer == "/derived" || pointer.starts_with("/derived/") => {
                let view = serde_json::to_value(SheetView::new(c)).map_err(err)?;
                let found = view
                    .pointer(pointer)
                    .ok_or_else(|| format!("no value at {pointer}"))?;
                to_json(&json!({ "pointer": pointer, "value": found }))
            }
            Some(pointer) => value_at(c, pointer),
        })
    }

    #[tool(
        description = "Every valid value for the sheet's choice fields: classes, armor, \
            abilities, skills, proficiency levels, conditions, item kinds, modifier kinds, \
            scopes, activations, costs, roll effects and recharges.",
        annotations(read_only_hint = true)
    )]
    async fn get_catalog(&self) -> ToolResult {
        let mut value = serde_json::to_value(catalog()).map_err(err)?;
        value["skills"] = json!(
            Skill::ALL
                .iter()
                .map(|s| json!({ "key": s, "name": s.name(), "ability": s.ability() }))
                .collect::<Vec<_>>()
        );
        value["proficiencies"] = json!([
            Proficiency::None,
            Proficiency::Proficient,
            Proficiency::Expertise
        ]);
        to_json(&value)
    }

    // --- Editing ------------------------------------------------------------------------

    #[tool(
        description = "Set the value at a JSON Pointer in the character, e.g. /hp/max or \
        /skills/stealth. Returns the stored value, which may be clamped to a valid range."
    )]
    async fn set_field(&self, Parameters(a): Parameters<SetArg>) -> ToolResult {
        self.update(&a.character, |c| {
            edit::set(c, &a.pointer, a.value).map_err(err)?;
            Ok((value_at(c, &a.pointer)?, Vec::new()))
        })
    }

    #[tool(
        description = "Append an entry to a list in the character, e.g. /inventory, \
        /spells, /features, /attacks, /classes, /effects or an entry's /modifiers or /actions. \
        Missing fields get defaults. Returns the new entry."
    )]
    async fn append_item(&self, Parameters(a): Parameters<AppendArg>) -> ToolResult {
        self.update(&a.character, |c| {
            let item = a.item.unwrap_or_else(|| json!({}));
            edit::push(c, &a.pointer, item).map_err(err)?;
            let len = serde_json::to_value(&*c)
                .map_err(err)?
                .pointer(&a.pointer)
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            Ok((
                value_at(c, &format!("{}/{}", a.pointer, len - 1))?,
                Vec::new(),
            ))
        })
    }

    #[tool(
        description = "Remove the entry at an index from a list in the character. Returns the \
            list afterwards.",
        annotations(destructive_hint = true)
    )]
    async fn remove_item(&self, Parameters(a): Parameters<RemoveArg>) -> ToolResult {
        self.update(&a.character, |c| {
            edit::remove(c, &a.pointer, a.index).map_err(err)?;
            Ok((value_at(c, &a.pointer)?, Vec::new()))
        })
    }

    #[tool(
        description = "Move a list entry from one index to another. Returns the list afterwards."
    )]
    async fn move_item(&self, Parameters(a): Parameters<MoveArg>) -> ToolResult {
        self.update(&a.character, |c| {
            edit::move_item(c, &a.pointer, a.from, a.to).map_err(err)?;
            Ok((value_at(c, &a.pointer)?, Vec::new()))
        })
    }

    #[tool(
        description = "Apply several edits in order, all or nothing. Each edit has an op \
        (set, append, remove or move) with the same fields as set_field, append_item, \
        remove_item and move_item. Read the result back with get_character."
    )]
    async fn apply_edits(&self, Parameters(a): Parameters<EditsArg>) -> ToolResult {
        self.update(&a.character, |c| {
            let n = a.edits.len();
            for (i, op) in a.edits.into_iter().enumerate() {
                let result = match op {
                    EditOp::Set { pointer, value } => edit::set(c, &pointer, value),
                    EditOp::Append { pointer, item } => {
                        edit::push(c, &pointer, item.unwrap_or_else(|| json!({})))
                    }
                    EditOp::Remove { pointer, index } => edit::remove(c, &pointer, index),
                    EditOp::Move { pointer, from, to } => edit::move_item(c, &pointer, from, to),
                };
                result.map_err(|e| format!("edit {i}: {e}; nothing was changed"))?;
            }
            Ok((format!("Applied {n} edits to {}", c.name), Vec::new()))
        })
    }

    // --- Hit points and rests -----------------------------------------------------------

    #[tool(description = "Take damage; temporary HP absorb it first.")]
    async fn damage(&self, Parameters(a): Parameters<AmountArg>) -> ToolResult {
        self.play(&a.character, |c| {
            c.take_damage(a.amount);
            Ok(())
        })
    }

    #[tool(
        description = "Regain hit points, up to the maximum. Healing from 0 HP resets death saves."
    )]
    async fn heal(&self, Parameters(a): Parameters<AmountArg>) -> ToolResult {
        self.play(&a.character, |c| {
            c.heal(a.amount);
            Ok(())
        })
    }

    #[tool(description = "Set temporary hit points, replacing the current amount.")]
    async fn set_temp_hp(&self, Parameters(a): Parameters<AmountArg>) -> ToolResult {
        self.play(&a.character, |c| {
            c.hp.temp = a.amount.max(0);
            Ok(())
        })
    }

    #[tool(description = "Spend a hit die: roll it, add the CON modifier and heal that much.")]
    async fn spend_hit_die(&self, Parameters(a): Parameters<HitDieArg>) -> ToolResult {
        self.update(&a.character, |c| {
            let message = play::spend_hit_die(c, a.class_index).map_err(play_err)?;
            let output = to_json(&json!({ "message": message, "status": status(c) }))?;
            Ok((output, Vec::new()))
        })
    }

    #[tool(description = "Take a short rest: recharges short-rest uses and Pact Magic slots.")]
    async fn short_rest(&self, Parameters(a): Parameters<CharacterArg>) -> ToolResult {
        self.play(&a.character, |c| {
            c.short_rest();
            Ok(())
        })
    }

    #[tool(
        description = "Take a long rest: full HP, all slots, uses and hit dice back, effects \
        end and Exhaustion drops by one."
    )]
    async fn long_rest(&self, Parameters(a): Parameters<CharacterArg>) -> ToolResult {
        self.play(&a.character, |c| {
            c.long_rest();
            Ok(())
        })
    }

    // --- Dice ---------------------------------------------------------------------------

    #[tool(description = "Roll dice and add the result to the character's roll log.")]
    async fn roll(&self, Parameters(a): Parameters<RollArg>) -> ToolResult {
        let result = dice::roll(&a.expression, a.mode.into()).map_err(err)?;
        let roll = StoredRoll::new(a.label.unwrap_or_else(|| a.expression.clone()), result);
        self.with_library(|lib| {
            lib.add_roll(find(lib, &a.character)?, &roll).map_err(err)?;
            to_json(&roll_summary(&roll))
        })
    }

    #[tool(
        description = "The character's recent rolls, newest first.",
        annotations(read_only_hint = true)
    )]
    async fn get_rolls(&self, Parameters(a): Parameters<CharacterArg>) -> ToolResult {
        self.read(&a.character, |lib, c| {
            let rolls = lib.recent_rolls(c.id).map_err(err)?;
            to_json(&rolls.iter().map(roll_summary).collect::<Vec<_>>())
        })
    }

    #[tool(
        description = "Clear the character's roll log.",
        annotations(destructive_hint = true)
    )]
    async fn clear_rolls(&self, Parameters(a): Parameters<CharacterArg>) -> ToolResult {
        self.with_library(|lib| {
            lib.clear_rolls(find(lib, &a.character)?).map_err(err)?;
            Ok("Cleared the roll log".to_owned())
        })
    }

    // --- Play ---------------------------------------------------------------------------

    #[tool(
        description = "Use one of an item's actions (character.inventory[index].actions): \
        spends its cost (charges or quantity), rolls its dice, heals or grants temp HP, and \
        applies its effect."
    )]
    async fn use_item_action(&self, Parameters(a): Parameters<ActionArg>) -> ToolResult {
        self.played(&a.character, |c| {
            play::use_item_action(c, a.index, a.action)
        })
    }

    #[tool(
        description = "Use one of a feature's actions (character.features[index].actions): \
        spends its uses, rolls its dice and applies its effect."
    )]
    async fn use_feature_action(&self, Parameters(a): Parameters<ActionArg>) -> ToolResult {
        self.played(&a.character, |c| {
            play::use_feature_action(c, a.index, a.action)
        })
    }

    #[tool(
        description = "Cast a spell from character.spells, spending a slot of slot_level (a \
        Pact Magic slot when no regular one is left) or a free cast when slot_level is 0. Rolls \
        its dice and applies its effect, ending any other concentration."
    )]
    async fn cast_spell(&self, Parameters(a): Parameters<CastArg>) -> ToolResult {
        self.played(&a.character, |c| play::cast_spell(c, a.spell, a.slot_level))
    }

    #[tool(
        description = "Roll to hit with an attack from derived.weapon_attacks. Firearms and \
        other ranged weapons spend a shot and tracked ammunition."
    )]
    async fn weapon_attack(&self, Parameters(a): Parameters<AttackArg>) -> ToolResult {
        self.rolled(&a.character, |c| {
            play::weapon_attack(c, a.attack, a.mode.into())
        })
    }

    #[tool(
        description = "Roll damage for an attack from derived.weapon_attacks, with its \
        riders (once-per-turn riders are then used up until new_turn)."
    )]
    async fn weapon_damage(&self, Parameters(a): Parameters<WeaponDamageArg>) -> ToolResult {
        self.rolled(&a.character, |c| {
            play::weapon_damage(c, a.attack, a.crit, &a.excluded_riders)
        })
    }

    #[tool(description = "Reload the weapon at character.inventory[index] to full capacity.")]
    async fn reload(&self, Parameters(a): Parameters<IndexArg>) -> ToolResult {
        self.play(&a.character, |c| c.reload(a.index).map_err(play_err))
    }

    #[tool(
        description = "Apply an effect to the character. A concentration effect ends any \
        other; one with the same name is replaced. rounds = 0 lasts until ended."
    )]
    async fn apply_effect(&self, Parameters(a): Parameters<EffectArg>) -> ToolResult {
        let effect: Effect = serde_json::from_value(a.effect).map_err(err)?;
        self.play(&a.character, |c| {
            c.apply_effect(effect);
            Ok(())
        })
    }

    #[tool(description = "End the effect at character.effects[index].")]
    async fn end_effect(&self, Parameters(a): Parameters<IndexArg>) -> ToolResult {
        self.play(&a.character, |c| {
            c.end_effect(a.index).map(|_| ()).map_err(play_err)
        })
    }

    #[tool(
        description = "Start the character's next turn: the round counter goes up, \
        once-per-turn riders reset and timed effects count down."
    )]
    async fn new_turn(&self, Parameters(a): Parameters<CharacterArg>) -> ToolResult {
        self.play(&a.character, |c| {
            c.new_turn();
            Ok(())
        })
    }

    #[tool(description = "Leave combat: the round counter resets and timed effects end.")]
    async fn end_combat(&self, Parameters(a): Parameters<CharacterArg>) -> ToolResult {
        self.play(&a.character, |c| {
            c.end_combat();
            Ok(())
        })
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CharSheetServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("charsheet", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> CharSheetServer {
        CharSheetServer::new(Library::open_in_memory().unwrap())
    }

    fn parse(text: ToolResult) -> Value {
        serde_json::from_str(&text.unwrap()).unwrap()
    }

    fn character(name: &str) -> CharacterArg {
        CharacterArg {
            character: name.into(),
        }
    }

    async fn import_sable(s: &CharSheetServer) -> Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sable.json");
        parse(
            s.import_character(Parameters(ImportArg {
                path: path.to_string_lossy().into_owned(),
            }))
            .await,
        )
    }

    async fn get(s: &CharSheetServer, pointer: &str) -> Value {
        parse(
            s.get_character(Parameters(GetArg {
                character: "sable".into(),
                pointer: Some(pointer.into()),
            }))
            .await,
        )["value"]
            .take()
    }

    async fn sheet(s: &CharSheetServer) -> Value {
        parse(
            s.get_character(Parameters(GetArg {
                character: "sable".into(),
                pointer: None,
            }))
            .await,
        )
    }

    #[tokio::test]
    async fn library_crud() {
        let s = server();
        let created = parse(
            s.create_character(Parameters(NameArg {
                name: "Brakka".into(),
            }))
            .await,
        );
        assert_eq!(created["level"], 1);
        parse(s.duplicate_character(Parameters(character("brakka"))).await);
        let names: Vec<_> = parse(s.list_characters().await)
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(names, ["Brakka", "Brakka (copy)"]);

        s.delete_character(Parameters(character("Brakka (copy)")))
            .await
            .unwrap();
        let by_id = created["id"].as_str().unwrap();
        s.get_character(Parameters(GetArg {
            character: by_id.into(),
            pointer: None,
        }))
        .await
        .unwrap();
        let missing = s.delete_character(Parameters(character("nobody"))).await;
        assert!(missing.unwrap_err().contains("No character named"));
    }

    #[tokio::test]
    async fn field_edits() {
        let s = server();
        import_sable(&s).await;
        let set = parse(
            s.set_field(Parameters(SetArg {
                character: "Sable".into(),
                pointer: "/hp/current".into(),
                value: json!(500),
            }))
            .await,
        );
        assert_eq!(set["value"], 52, "clamped to max HP");

        let added = parse(
            s.append_item(Parameters(AppendArg {
                character: "Sable".into(),
                pointer: "/inventory".into(),
                item: Some(json!({ "name": "Lantern", "quantity": 2 })),
            }))
            .await,
        );
        assert_eq!(added["value"]["name"], "Lantern");
        assert_eq!(added["value"]["kind"], "gear");
        let count = get(&s, "/inventory").await.as_array().unwrap().len();

        s.remove_item(Parameters(RemoveArg {
            character: "Sable".into(),
            pointer: "/inventory".into(),
            index: count - 1,
        }))
        .await
        .unwrap();
        assert_eq!(
            get(&s, "/inventory").await.as_array().unwrap().len(),
            count - 1
        );
        assert_eq!(get(&s, "/derived/armor_class").await, 16);
    }

    #[tokio::test]
    async fn bad_edits_change_nothing() {
        let s = server();
        import_sable(&s).await;
        let before = sheet(&s).await;
        let bad = s
            .apply_edits(Parameters(EditsArg {
                character: "Sable".into(),
                edits: vec![
                    EditOp::Set {
                        pointer: "/name".into(),
                        value: json!("Renamed"),
                    },
                    EditOp::Set {
                        pointer: "/hp/max".into(),
                        value: json!("lots"),
                    },
                ],
            }))
            .await;
        assert!(bad.unwrap_err().starts_with("edit 1:"));
        assert!(
            s.set_field(Parameters(SetArg {
                character: "Sable".into(),
                pointer: "/no/such/field".into(),
                value: json!(1),
            }))
            .await
            .is_err()
        );
        assert_eq!(sheet(&s).await, before);
    }

    #[tokio::test]
    async fn play_and_rest() {
        let s = server();
        import_sable(&s).await;
        let hit = parse(
            s.damage(Parameters(AmountArg {
                character: "Sable".into(),
                amount: 30,
            }))
            .await,
        );
        assert_eq!(hit["status"]["hp"]["current"], 22);

        let cure = get(&s, "/spells")
            .await
            .as_array()
            .unwrap()
            .iter()
            .position(|sp| sp["name"] == "Cure Wounds")
            .unwrap();
        let cast = parse(
            s.cast_spell(Parameters(CastArg {
                character: "Sable".into(),
                spell: cure,
                slot_level: 1,
            }))
            .await,
        );
        assert_eq!(cast["status"]["spell_slots"][0]["left"], 3);
        assert_eq!(cast["rolls"][0]["label"], "Cure Wounds");

        let attack = parse(
            s.weapon_attack(Parameters(AttackArg {
                character: "Sable".into(),
                attack: 0,
                mode: Mode::Advantage,
            }))
            .await,
        );
        assert!(
            attack["roll"]["label"]
                .as_str()
                .unwrap()
                .ends_with("attack")
        );
        let rolls = parse(s.get_rolls(Parameters(character("Sable"))).await);
        assert_eq!(rolls.as_array().unwrap().len(), 2);

        let rested = parse(s.long_rest(Parameters(character("Sable"))).await);
        assert_eq!(rested["status"]["hp"]["current"], 52);
        assert_eq!(rested["status"]["spell_slots"][0]["left"], 4);
    }

    #[test]
    fn every_tool_is_listed() {
        assert_eq!(server().tool_router.list_all().len(), 32);
    }
}
