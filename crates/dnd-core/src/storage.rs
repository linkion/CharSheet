//! The character library: one SQLite database holding every character, with a table per
//! kind of sheet entry, plus each character's recent dice rolls. Single characters move in
//! and out of the library as JSON files (import/export).

use crate::abilities::Ability;
use crate::character::{
    Attack, Character, ClassLevel, Currency, DeathSaves, Feature, HitPoints, Item, SCHEMA_VERSION,
    Spell,
};
use crate::dice::RollResult;
use crate::effects::Effect;
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, named_params, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

/// How many rolls are kept per character.
pub const MAX_ROLLS: usize = 50;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("not a valid character file: {0}")]
    Format(#[from] serde_json::Error),
    #[error("database error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("this character was saved by a newer version of the app (format {0})")]
    TooNew(u32),
    #[error("the character database was created by a newer version of the app (schema {0})")]
    DatabaseTooNew(u32),
    #[error("no character with id {0}")]
    NotFound(Uuid),
    #[error("no data directory available")]
    NoDataDir,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Summary {
    pub id: Uuid,
    pub name: String,
    pub species: String,
    pub class_summary: String,
    pub level: u32,
}

impl From<&Character> for Summary {
    fn from(c: &Character) -> Self {
        Summary {
            id: c.id,
            name: c.name.clone(),
            species: c.species.clone(),
            class_summary: c.class_summary(),
            level: c.total_level(),
        }
    }
}

/// A roll in a character's log, as shown in the roll log drawer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredRoll {
    /// The roll's row in the log, 0 until it's saved. Newer rolls have larger ids.
    #[serde(default)]
    pub id: i64,
    pub label: String,
    pub breakdown: String,
    /// When it was logged, in UTC as `2026-10-31T18:04:05.123Z`; empty until it's saved.
    #[serde(default)]
    pub rolled_at: String,
    #[serde(flatten)]
    pub result: RollResult,
}

impl StoredRoll {
    pub fn new(label: impl Into<String>, result: RollResult) -> StoredRoll {
        StoredRoll {
            id: 0,
            label: label.into(),
            breakdown: result.breakdown(),
            rolled_at: String::new(),
            result,
        }
    }
}

/// Schema migrations from `migrations/`, applied in order; `PRAGMA user_version` counts how
/// many have run. Add a new numbered `.sql` file for each change and append it here.
const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_initial.sql"),
    include_str!("../migrations/0002_items_effects.sql"),
    include_str!("../migrations/0003_revision.sql"),
    include_str!("../migrations/0004_formulas.sql"),
    include_str!("../migrations/0005_spell_damage_type.sql"),
];

/// Tables holding a character's sheet entries, cleared and rewritten on every save.
const CHILD_TABLES: &[&str] = &[
    "character_classes",
    "character_abilities",
    "character_skills",
    "character_conditions",
    "character_spell_slots",
    "attacks",
    "spells",
    "items",
    "features",
    "effects",
];

/// Columns of `characters` other than `id`, the timestamps and the revision; also the save
/// parameter names.
const CHARACTER_COLUMNS: &[&str] = &[
    "name",
    "player",
    "species",
    "background",
    "alignment",
    "jack_of_all_trades",
    "hp_max",
    "hp_current",
    "hp_temp",
    "death_successes",
    "death_failures",
    "heroic_inspiration",
    "exhaustion",
    "speed",
    "armor",
    "shield",
    "ac_bonus",
    "initiative_bonus",
    "pact_slots_used",
    "cp",
    "sp",
    "ep",
    "gp",
    "pp",
    "languages",
    "other_proficiencies",
    "appearance",
    "backstory",
    "notes",
    "round",
    "riders_used_json",
];

/// An enum's serde name (`"str"`, `"chain_mail"`), so the database matches the JSON format.
fn to_text<T: Serialize>(value: &T) -> Result<String, StorageError> {
    Ok(match serde_json::to_value(value)? {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    })
}

fn from_text<T: DeserializeOwned>(text: String, column: usize) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(text))
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(e)))
}

fn text_col<T: DeserializeOwned>(row: &Row, column: usize) -> rusqlite::Result<T> {
    from_text(row.get(column)?, column)
}

/// A nested value stored as JSON text, like an item's actions.
fn json_col<T: DeserializeOwned>(row: &Row, column: usize) -> rusqlite::Result<T> {
    let text: String = row.get(column)?;
    serde_json::from_str(&text)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(e)))
}

/// Like `json_col`, for a nullable column holding an optional value.
fn opt_json_col<T: DeserializeOwned>(row: &Row, column: usize) -> rusqlite::Result<Option<T>> {
    let text: Option<String> = row.get(column)?;
    text.map(|t| {
        serde_json::from_str(&t)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(e)))
    })
    .transpose()
}

pub struct Library {
    conn: Connection,
}

/// Reads a character from any file, upgrading older formats.
pub fn read_file(path: &Path) -> Result<Character, StorageError> {
    let text = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    let version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;
    if version > SCHEMA_VERSION {
        return Err(StorageError::TooNew(version));
    }
    // Migrations from older schema versions go here.
    let mut character: Character = serde_json::from_value(value)?;
    character.schema_version = SCHEMA_VERSION;
    character.normalize();
    Ok(character)
}

/// Writes atomically: to a temporary file in the same directory, then renames it.
pub fn write_file(path: &Path, character: &Character) -> Result<(), StorageError> {
    let json = serde_json::to_string_pretty(character)?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

impl Library {
    /// The default library at `$XDG_DATA_HOME/charsheet/charsheet.db`.
    pub fn open_default() -> Result<Library, StorageError> {
        let path = dirs::data_dir()
            .ok_or(StorageError::NoDataDir)?
            .join("charsheet")
            .join("charsheet.db");
        Library::open(path)
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Library, StorageError> {
        let path = path.into();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(&path)?;
        // The library list and the open sheet each hold a connection.
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        Library::init(conn)
    }

    /// A throwaway library, for tests.
    pub fn open_in_memory() -> Result<Library, StorageError> {
        Library::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Library, StorageError> {
        conn.pragma_update(None, "foreign_keys", true)?;
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version as usize > MIGRATIONS.len() {
            return Err(StorageError::DatabaseTooNew(version));
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i as u32 + 1)?;
            tx.commit()?;
        }
        Ok(Library { conn })
    }

    fn exists(&self, id: Uuid) -> Result<bool, StorageError> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM characters WHERE id = ?1",
                [id.to_string()],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// Bumps the character's revision, marking it changed for other connections.
    fn touch(conn: &Connection, key: &str) -> Result<(), StorageError> {
        conn.execute(
            "UPDATE characters SET revision = revision + 1 WHERE id = ?1",
            [key],
        )?;
        Ok(())
    }

    /// The character's revision, which goes up with every save and every roll logged.
    pub fn revision(&self, id: Uuid) -> Result<i64, StorageError> {
        self.conn
            .query_row(
                "SELECT revision FROM characters WHERE id = ?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(StorageError::NotFound(id))
    }

    /// Changes whenever another connection (another window or process) commits a write, so
    /// it's a cheap check before looking for what changed.
    pub fn data_version(&self) -> Result<i64, StorageError> {
        Ok(self
            .conn
            .pragma_query_value(None, "data_version", |r| r.get(0))?)
    }

    /// Every readable character, sorted by name. Characters that can't be read are skipped.
    pub fn list(&self) -> Result<Vec<Summary>, StorageError> {
        let ids = self
            .conn
            .prepare("SELECT id FROM characters ORDER BY name COLLATE NOCASE, id")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut out = Vec::new();
        for id in ids {
            match Uuid::parse_str(&id)
                .map_err(|e| StorageError::Sql(rusqlite::Error::ToSqlConversionFailure(e.into())))
                .and_then(|id| self.load(id))
            {
                Ok(c) => out.push(Summary::from(&c)),
                Err(e) => eprintln!("skipping character {id}: {e}"),
            }
        }
        Ok(out)
    }

    pub fn load(&self, id: Uuid) -> Result<Character, StorageError> {
        let key = id.to_string();
        let conn = &self.conn;
        let tx = conn.unchecked_transaction()?;

        let mut character = tx
            .query_row("SELECT * FROM characters WHERE id = ?1", [&key], |r| {
                let armor: Option<String> = r.get("armor")?;
                let armor_col = r.as_ref().column_index("armor")?;
                Ok(Character {
                    schema_version: SCHEMA_VERSION,
                    id,
                    name: r.get("name")?,
                    player: r.get("player")?,
                    species: r.get("species")?,
                    background: r.get("background")?,
                    alignment: r.get("alignment")?,
                    classes: Vec::new(),
                    base_scores: [10; 6],
                    score_bonuses: [0; 6],
                    extra_saves: Default::default(),
                    skills: Default::default(),
                    jack_of_all_trades: r.get("jack_of_all_trades")?,
                    hp: HitPoints {
                        max: r.get("hp_max")?,
                        current: r.get("hp_current")?,
                        temp: r.get("hp_temp")?,
                    },
                    death_saves: DeathSaves {
                        successes: r.get("death_successes")?,
                        failures: r.get("death_failures")?,
                    },
                    heroic_inspiration: r.get("heroic_inspiration")?,
                    exhaustion: r.get("exhaustion")?,
                    conditions: Default::default(),
                    speed: r.get("speed")?,
                    armor: armor.map(|a| from_text(a, armor_col)).transpose()?,
                    shield: r.get("shield")?,
                    ac_bonus: r.get("ac_bonus")?,
                    initiative_bonus: r.get("initiative_bonus")?,
                    attacks: Vec::new(),
                    spells: Vec::new(),
                    spell_slots_used: [0; 9],
                    pact_slots_used: r.get("pact_slots_used")?,
                    currency: Currency {
                        cp: r.get("cp")?,
                        sp: r.get("sp")?,
                        ep: r.get("ep")?,
                        gp: r.get("gp")?,
                        pp: r.get("pp")?,
                    },
                    inventory: Vec::new(),
                    features: Vec::new(),
                    languages: r.get("languages")?,
                    other_proficiencies: r.get("other_proficiencies")?,
                    appearance: r.get("appearance")?,
                    backstory: r.get("backstory")?,
                    notes: r.get("notes")?,
                    effects: Vec::new(),
                    round: r.get("round")?,
                    riders_used: json_col(r, r.as_ref().column_index("riders_used_json")?)?,
                })
            })
            .optional()?
            .ok_or(StorageError::NotFound(id))?;

        character.classes = tx
            .prepare(
                "SELECT class, subclass, level, hit_dice_spent, third_caster
                 FROM character_classes WHERE character_id = ?1 ORDER BY position",
            )?
            .query_map([&key], |r| {
                Ok(ClassLevel {
                    class: text_col(r, 0)?,
                    subclass: r.get(1)?,
                    level: r.get(2)?,
                    hit_dice_spent: r.get(3)?,
                    third_caster: r.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        {
            let mut stmt = tx.prepare(
                "SELECT ability, base_score, bonus, extra_save
                 FROM character_abilities WHERE character_id = ?1",
            )?;
            let mut rows = stmt.query([&key])?;
            while let Some(r) = rows.next()? {
                let ability: Ability = text_col(r, 0)?;
                character.base_scores[ability.index()] = r.get(1)?;
                character.score_bonuses[ability.index()] = r.get(2)?;
                if r.get(3)? {
                    character.extra_saves.insert(ability);
                }
            }
        }

        character.skills = tx
            .prepare("SELECT skill, proficiency FROM character_skills WHERE character_id = ?1")?
            .query_map([&key], |r| Ok((text_col(r, 0)?, text_col(r, 1)?)))?
            .collect::<Result<_, _>>()?;

        character.conditions = tx
            .prepare("SELECT condition FROM character_conditions WHERE character_id = ?1")?
            .query_map([&key], |r| text_col(r, 0))?
            .collect::<Result<_, _>>()?;

        {
            let mut stmt = tx
                .prepare("SELECT level, used FROM character_spell_slots WHERE character_id = ?1")?;
            let mut rows = stmt.query([&key])?;
            while let Some(r) = rows.next()? {
                let level: u32 = r.get(0)?;
                if let Some(slot) = character.spell_slots_used.get_mut(level as usize - 1) {
                    *slot = r.get(1)?;
                }
            }
        }

        character.attacks = tx
            .prepare(
                "SELECT name, ability, proficient, damage, damage_type, bonus,
                        add_ability_to_damage, mastery, notes
                 FROM attacks WHERE character_id = ?1 ORDER BY position",
            )?
            .query_map([&key], |r| {
                Ok(Attack {
                    name: r.get(0)?,
                    ability: text_col(r, 1)?,
                    proficient: r.get(2)?,
                    damage: r.get(3)?,
                    damage_type: r.get(4)?,
                    bonus: r.get(5)?,
                    add_ability_to_damage: r.get(6)?,
                    mastery: r.get(7)?,
                    notes: r.get(8)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        character.spells = tx
            .prepare(
                "SELECT name, level, prepared, ritual, concentration, notes, casting_time,
                        free_uses_max, free_uses_spent, roll, roll_effect, effect_json,
                        roll_formula, damage_type
                 FROM spells WHERE character_id = ?1 ORDER BY position",
            )?
            .query_map([&key], |r| {
                Ok(Spell {
                    name: r.get(0)?,
                    level: r.get(1)?,
                    prepared: r.get(2)?,
                    ritual: r.get(3)?,
                    concentration: r.get(4)?,
                    notes: r.get(5)?,
                    casting_time: text_col(r, 6)?,
                    free_uses_max: r.get(7)?,
                    free_uses_spent: r.get(8)?,
                    roll: r.get(9)?,
                    roll_effect: text_col(r, 10)?,
                    effect: opt_json_col(r, 11)?,
                    roll_formula: r.get(12)?,
                    damage_type: r.get(13)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        character.inventory = tx
            .prepare(
                "SELECT name, quantity, weight, requires_attunement, attuned, notes, kind,
                        equipped, weapon_json, armor, ac_bonus, charges_max, charges_spent,
                        recharge, modifiers_json, actions_json, regain
                 FROM items WHERE character_id = ?1 ORDER BY position",
            )?
            .query_map([&key], |r| {
                let armor: Option<String> = r.get(9)?;
                Ok(Item {
                    name: r.get(0)?,
                    quantity: r.get(1)?,
                    weight: r.get(2)?,
                    requires_attunement: r.get(3)?,
                    attuned: r.get(4)?,
                    notes: r.get(5)?,
                    kind: text_col(r, 6)?,
                    equipped: r.get(7)?,
                    weapon: opt_json_col(r, 8)?,
                    armor: armor.map(|a| from_text(a, 9)).transpose()?,
                    ac_bonus: r.get(10)?,
                    charges_max: r.get(11)?,
                    charges_spent: r.get(12)?,
                    recharge: text_col(r, 13)?,
                    modifiers: json_col(r, 14)?,
                    actions: json_col(r, 15)?,
                    regain: r.get(16)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        character.features = tx
            .prepare(
                "SELECT name, source, description, uses_max, uses_spent, recharge,
                        modifiers_json, actions_json, uses_formula
                 FROM features WHERE character_id = ?1 ORDER BY position",
            )?
            .query_map([&key], |r| {
                Ok(Feature {
                    name: r.get(0)?,
                    source: r.get(1)?,
                    description: r.get(2)?,
                    uses_max: r.get(3)?,
                    uses_spent: r.get(4)?,
                    recharge: text_col(r, 5)?,
                    modifiers: json_col(r, 6)?,
                    actions: json_col(r, 7)?,
                    uses_formula: r.get(8)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        character.effects = tx
            .prepare(
                "SELECT name, source, concentration, rounds, modifiers_json, notes
                 FROM effects WHERE character_id = ?1 ORDER BY position",
            )?
            .query_map([&key], |r| {
                Ok(Effect {
                    name: r.get(0)?,
                    source: r.get(1)?,
                    concentration: r.get(2)?,
                    rounds: r.get(3)?,
                    modifiers: json_col(r, 4)?,
                    notes: r.get(5)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        tx.commit()?;
        character.normalize();
        Ok(character)
    }

    /// Writes the whole character in one transaction, replacing its sheet entries. Returns
    /// its new revision.
    pub fn save(&self, character: &Character) -> Result<i64, StorageError> {
        // Destructured so that a new field can't be forgotten here.
        let Character {
            schema_version: _,
            id,
            name,
            player,
            species,
            background,
            alignment,
            classes,
            base_scores,
            score_bonuses,
            extra_saves,
            skills,
            jack_of_all_trades,
            hp,
            death_saves,
            heroic_inspiration,
            exhaustion,
            conditions,
            speed,
            armor,
            shield,
            ac_bonus,
            initiative_bonus,
            attacks,
            spells,
            spell_slots_used,
            pact_slots_used,
            currency,
            inventory,
            features,
            languages,
            other_proficiencies,
            appearance,
            backstory,
            notes,
            effects,
            round,
            riders_used,
        } = character;
        let key = id.to_string();
        let armor = armor.as_ref().map(to_text).transpose()?;
        let riders_used = serde_json::to_string(riders_used)?;
        let tx = self.conn.unchecked_transaction()?;

        let values = named_params! {
            ":id": key,
            ":name": name,
            ":player": player,
            ":species": species,
            ":background": background,
            ":alignment": alignment,
            ":jack_of_all_trades": jack_of_all_trades,
            ":hp_max": hp.max,
            ":hp_current": hp.current,
            ":hp_temp": hp.temp,
            ":death_successes": death_saves.successes,
            ":death_failures": death_saves.failures,
            ":heroic_inspiration": heroic_inspiration,
            ":exhaustion": exhaustion,
            ":speed": speed,
            ":armor": armor,
            ":shield": shield,
            ":ac_bonus": ac_bonus,
            ":initiative_bonus": initiative_bonus,
            ":pact_slots_used": pact_slots_used,
            ":cp": currency.cp,
            ":sp": currency.sp,
            ":ep": currency.ep,
            ":gp": currency.gp,
            ":pp": currency.pp,
            ":languages": languages,
            ":other_proficiencies": other_proficiencies,
            ":appearance": appearance,
            ":backstory": backstory,
            ":notes": notes,
            ":round": round,
            ":riders_used_json": riders_used,
        };
        // Update in place rather than REPLACE, which would cascade-delete the roll log.
        let assignments: Vec<String> = CHARACTER_COLUMNS
            .iter()
            .map(|c| format!("{c} = :{c}"))
            .collect();
        let updated: Option<i64> = tx
            .prepare_cached(&format!(
                "UPDATE characters SET {}, revision = revision + 1,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = :id RETURNING revision",
                assignments.join(", ")
            ))?
            .query_row(values, |r| r.get(0))
            .optional()?;
        let revision = match updated {
            Some(revision) => revision,
            None => {
                tx.prepare_cached(&format!(
                    "INSERT INTO characters (id, {}) VALUES (:id, :{})",
                    CHARACTER_COLUMNS.join(", "),
                    CHARACTER_COLUMNS.join(", :")
                ))?
                .execute(values)?;
                0
            }
        };

        for table in CHILD_TABLES {
            tx.execute(
                &format!("DELETE FROM {table} WHERE character_id = ?1"),
                [&key],
            )?;
        }

        {
            let mut stmt = tx.prepare(
                "INSERT INTO character_classes
                 (character_id, position, class, subclass, level, hit_dice_spent, third_caster)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for (i, c) in classes.iter().enumerate() {
                stmt.execute(params![
                    key,
                    i as i64,
                    to_text(&c.class)?,
                    c.subclass,
                    c.level,
                    c.hit_dice_spent,
                    c.third_caster
                ])?;
            }

            let mut stmt = tx.prepare(
                "INSERT INTO character_abilities
                 (character_id, ability, base_score, bonus, extra_save)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for ability in Ability::ALL {
                stmt.execute(params![
                    key,
                    to_text(&ability)?,
                    base_scores[ability.index()],
                    score_bonuses[ability.index()],
                    extra_saves.contains(&ability)
                ])?;
            }

            let mut stmt = tx.prepare(
                "INSERT INTO character_skills (character_id, skill, proficiency)
                 VALUES (?1, ?2, ?3)",
            )?;
            for (skill, proficiency) in skills {
                stmt.execute(params![key, to_text(skill)?, to_text(proficiency)?])?;
            }

            let mut stmt = tx.prepare(
                "INSERT INTO character_conditions (character_id, condition) VALUES (?1, ?2)",
            )?;
            for condition in conditions {
                stmt.execute(params![key, to_text(condition)?])?;
            }

            let mut stmt = tx.prepare(
                "INSERT INTO character_spell_slots (character_id, level, used) VALUES (?1, ?2, ?3)",
            )?;
            for (i, used) in spell_slots_used.iter().enumerate() {
                stmt.execute(params![key, i as u32 + 1, used])?;
            }

            let mut stmt = tx.prepare(
                "INSERT INTO attacks
                 (character_id, position, name, ability, proficient, damage, damage_type, bonus,
                  add_ability_to_damage, mastery, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for (i, a) in attacks.iter().enumerate() {
                stmt.execute(params![
                    key,
                    i as i64,
                    a.name,
                    to_text(&a.ability)?,
                    a.proficient,
                    a.damage,
                    a.damage_type,
                    a.bonus,
                    a.add_ability_to_damage,
                    a.mastery,
                    a.notes
                ])?;
            }

            let mut stmt = tx.prepare(
                "INSERT INTO spells
                 (character_id, position, name, level, prepared, ritual, concentration, notes,
                  casting_time, free_uses_max, free_uses_spent, roll, roll_effect, effect_json,
                  roll_formula, damage_type)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            )?;
            for (i, s) in spells.iter().enumerate() {
                stmt.execute(params![
                    key,
                    i as i64,
                    s.name,
                    s.level,
                    s.prepared,
                    s.ritual,
                    s.concentration,
                    s.notes,
                    to_text(&s.casting_time)?,
                    s.free_uses_max,
                    s.free_uses_spent,
                    s.roll,
                    to_text(&s.roll_effect)?,
                    s.effect.as_ref().map(serde_json::to_string).transpose()?,
                    s.roll_formula,
                    s.damage_type
                ])?;
            }

            let mut stmt = tx.prepare(
                "INSERT INTO items
                 (character_id, position, name, quantity, weight, requires_attunement, attuned, notes,
                  kind, equipped, weapon_json, armor, ac_bonus, charges_max, charges_spent,
                  recharge, modifiers_json, actions_json, regain)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                         ?17, ?18, ?19)",
            )?;
            for (i, item) in inventory.iter().enumerate() {
                stmt.execute(params![
                    key,
                    i as i64,
                    item.name,
                    item.quantity,
                    item.weight,
                    item.requires_attunement,
                    item.attuned,
                    item.notes,
                    to_text(&item.kind)?,
                    item.equipped,
                    item.weapon
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()?,
                    item.armor.as_ref().map(to_text).transpose()?,
                    item.ac_bonus,
                    item.charges_max,
                    item.charges_spent,
                    to_text(&item.recharge)?,
                    serde_json::to_string(&item.modifiers)?,
                    serde_json::to_string(&item.actions)?,
                    item.regain
                ])?;
            }

            let mut stmt = tx.prepare(
                "INSERT INTO features
                 (character_id, position, name, source, description, uses_max, uses_spent, recharge,
                  modifiers_json, actions_json, uses_formula)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for (i, f) in features.iter().enumerate() {
                stmt.execute(params![
                    key,
                    i as i64,
                    f.name,
                    f.source,
                    f.description,
                    f.uses_max,
                    f.uses_spent,
                    to_text(&f.recharge)?,
                    serde_json::to_string(&f.modifiers)?,
                    serde_json::to_string(&f.actions)?,
                    f.uses_formula
                ])?;
            }

            let mut stmt = tx.prepare(
                "INSERT INTO effects
                 (character_id, position, name, source, concentration, rounds, modifiers_json, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for (i, e) in effects.iter().enumerate() {
                stmt.execute(params![
                    key,
                    i as i64,
                    e.name,
                    e.source,
                    e.concentration,
                    e.rounds,
                    serde_json::to_string(&e.modifiers)?,
                    e.notes
                ])?;
            }
        }

        tx.commit()?;
        Ok(revision)
    }

    /// Deletes the character with its sheet entries and roll log.
    pub fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        let deleted = self
            .conn
            .execute("DELETE FROM characters WHERE id = ?1", [id.to_string()])?;
        if deleted == 0 {
            return Err(StorageError::NotFound(id));
        }
        Ok(())
    }

    pub fn create(&self, name: &str) -> Result<Character, StorageError> {
        let character = Character::new(name);
        self.save(&character)?;
        Ok(character)
    }

    pub fn duplicate(&self, id: Uuid) -> Result<Character, StorageError> {
        let mut copy = self.load(id)?;
        copy.id = Uuid::new_v4();
        copy.name = format!("{} (copy)", copy.name);
        self.save(&copy)?;
        Ok(copy)
    }

    /// Imports a file into the library. It gets a new id if that id is already taken.
    pub fn import(&self, path: &Path) -> Result<Character, StorageError> {
        let mut character = read_file(path)?;
        if self.exists(character.id)? {
            character.id = Uuid::new_v4();
        }
        self.save(&character)?;
        Ok(character)
    }

    pub fn export(&self, id: Uuid, path: &Path) -> Result<(), StorageError> {
        write_file(path, &self.load(id)?)
    }

    /// Adds a roll to the character's log, dropping all but the newest `MAX_ROLLS`.
    /// Returns the roll's id.
    pub fn add_roll(&self, id: Uuid, roll: &StoredRoll) -> Result<i64, StorageError> {
        let key = id.to_string();
        let r = &roll.result;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO rolls
             (character_id, label, expression, mode, total, natural, breakdown, terms_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                key,
                roll.label,
                r.expression,
                to_text(&r.mode)?,
                r.total,
                r.natural,
                roll.breakdown,
                serde_json::to_string(&r.terms)?
            ],
        )?;
        let roll_id = tx.last_insert_rowid();
        tx.execute(
            "DELETE FROM rolls WHERE character_id = ?1 AND id NOT IN
             (SELECT id FROM rolls WHERE character_id = ?1 ORDER BY id DESC LIMIT ?2)",
            params![key, MAX_ROLLS as u32],
        )?;
        Library::touch(&tx, &key)?;
        tx.commit()?;
        Ok(roll_id)
    }

    /// When the logged roll with this id was made.
    pub fn rolled_at(&self, roll_id: i64) -> Result<String, StorageError> {
        Ok(self.conn.query_row(
            "SELECT rolled_at FROM rolls WHERE id = ?1",
            [roll_id],
            |r| r.get(0),
        )?)
    }

    /// The character's logged rolls, newest first.
    pub fn recent_rolls(&self, id: Uuid) -> Result<Vec<StoredRoll>, StorageError> {
        let rows = self
            .conn
            .prepare(
                "SELECT label, breakdown, expression, mode, total, natural, terms_json, id, rolled_at
                 FROM rolls WHERE character_id = ?1 ORDER BY id DESC LIMIT ?2",
            )?
            .query_map(params![id.to_string(), MAX_ROLLS as u32], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    RollResult {
                        expression: r.get(2)?,
                        mode: text_col(r, 3)?,
                        terms: Vec::new(),
                        total: r.get(4)?,
                        natural: r.get(5)?,
                    },
                    r.get::<_, String>(6)?,
                    r.get::<_, i64>(7)?,
                    r.get::<_, String>(8)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(label, breakdown, mut result, terms, id, rolled_at)| {
                result.terms = serde_json::from_str(&terms)?;
                Ok(StoredRoll {
                    id,
                    label,
                    breakdown,
                    rolled_at,
                    result,
                })
            })
            .collect()
    }

    pub fn clear_rolls(&self, id: Uuid) -> Result<(), StorageError> {
        let key = id.to_string();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM rolls WHERE character_id = ?1", [&key])?;
        Library::touch(&tx, &key)?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{Proficiency, Skill};
    use crate::armor::ArmorKind;
    use crate::character::{Condition, ItemKind, Recharge, WeaponStats};
    use crate::classes::ClassId;
    use crate::dice::{self, RollMode};
    use crate::effects::{Action, Activation, Cost, Modifier, ModifierKind, RollEffect, Scope};

    fn count(lib: &Library, table: &str) -> i64 {
        lib.conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    /// A character with something in every table.
    fn populated() -> Character {
        let mut c = Character::new("Zed");
        c.player = "Pat".into();
        c.species = "Elf".into();
        c.classes = vec![
            ClassLevel {
                class: ClassId::Rogue,
                subclass: "Arcane Trickster".into(),
                level: 5,
                hit_dice_spent: 2,
                third_caster: true,
            },
            ClassLevel {
                class: ClassId::Wizard,
                level: 2,
                ..Default::default()
            },
        ];
        c.base_scores = [8, 16, 14, 15, 12, 10];
        c.score_bonuses = [0, 2, 0, 1, 0, 0];
        c.extra_saves.insert(Ability::Con);
        c.skills.insert(Skill::Stealth, Proficiency::Expertise);
        c.skills
            .insert(Skill::AnimalHandling, Proficiency::Proficient);
        c.jack_of_all_trades = true;
        c.hp = HitPoints {
            max: 40,
            current: 31,
            temp: 5,
        };
        c.death_saves = DeathSaves {
            successes: 1,
            failures: 2,
        };
        c.heroic_inspiration = true;
        c.exhaustion = 2;
        c.conditions.insert(Condition::Prone);
        c.conditions.insert(Condition::Poisoned);
        c.armor = Some(ArmorKind::StuddedLeather);
        c.shield = true;
        c.ac_bonus = 1;
        c.initiative_bonus = 2;
        c.attacks = vec![
            Attack {
                name: "Rapier".into(),
                ability: Ability::Dex,
                damage: "1d8".into(),
                damage_type: "piercing".into(),
                bonus: 1,
                mastery: "Vex".into(),
                ..Default::default()
            },
            Attack {
                name: "Dagger".into(),
                ..Default::default()
            },
        ];
        c.spells = vec![
            Spell {
                name: "Mage Hand".into(),
                ..Default::default()
            },
            Spell {
                name: "Shield".into(),
                level: 1,
                prepared: true,
                ..Default::default()
            },
            Spell {
                name: "Hunter's Mark".into(),
                level: 1,
                prepared: true,
                concentration: true,
                casting_time: Activation::BonusAction,
                free_uses_max: 3,
                free_uses_spent: 1,
                roll: "1d6".into(),
                roll_effect: RollEffect::TempHp,
                effect: Some(Effect {
                    name: "Marked".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];
        c.spell_slots_used[0] = 1;
        c.currency = Currency {
            cp: 3,
            sp: 4,
            ep: 0,
            gp: 125,
            pp: 1,
        };
        c.inventory.push(Item {
            name: "Rope".into(),
            quantity: 2,
            weight: 2.5,
            requires_attunement: true,
            attuned: true,
            notes: "hempen".into(),
            ..Default::default()
        });
        c.inventory.push(Item {
            name: "Revolver".into(),
            kind: ItemKind::Weapon,
            equipped: true,
            weapon: Some(WeaponStats {
                ranged: true,
                capacity: 6,
                loaded: 4,
                ammo: "Rounds".into(),
                mastery: "Vex".into(),
                ..Default::default()
            }),
            charges_max: 3,
            charges_spent: 1,
            recharge: Recharge::Long,
            modifiers: vec![Modifier {
                kind: ModifierKind::Damage,
                scope: Scope::Ranged,
                dice: "1d4".into(),
                once_per_turn: true,
                ..Default::default()
            }],
            actions: vec![Action {
                name: "Fan the hammer".into(),
                cost: Cost::Charges,
                ..Default::default()
            }],
            ..Default::default()
        });
        c.inventory.push(Item {
            name: "Breastplate".into(),
            kind: ItemKind::Armor,
            armor: Some(ArmorKind::Breastplate),
            ac_bonus: 1,
            ..Default::default()
        });
        c.features.push(Feature {
            name: "Cunning Action".into(),
            source: "Rogue".into(),
            description: "Dash, Disengage or Hide".into(),
            uses_max: 3,
            uses_formula: "{pb}".into(),
            uses_spent: 1,
            recharge: Recharge::Short,
            modifiers: vec![Modifier {
                kind: ModifierKind::Save,
                ability: Some(Ability::Dex),
                value: 1,
                ..Default::default()
            }],
            actions: vec![Action {
                name: "Hide".into(),
                activation: Activation::BonusAction,
                ..Default::default()
            }],
        });
        c.effects.push(Effect {
            name: "Bless".into(),
            source: "Cleric".into(),
            concentration: true,
            rounds: 10,
            modifiers: vec![Modifier {
                kind: ModifierKind::Note,
                text: "+1d4 to attacks and saves".into(),
                ..Default::default()
            }],
            notes: "From the party cleric".into(),
        });
        c.round = 3;
        c.riders_used.insert("Sneak Attack".into());
        c.backstory = "Grew up in the woods.".into();
        c.normalize();
        c
    }

    #[test]
    fn round_trip_every_field() {
        let lib = Library::open_in_memory().unwrap();
        let mut c = populated();
        lib.save(&c).unwrap();
        assert_eq!(lib.load(c.id).unwrap(), c);

        // Saving again replaces entries rather than adding to them.
        c.attacks.swap(0, 1);
        c.armor = None;
        c.conditions.clear();
        c.inventory.clear();
        lib.save(&c).unwrap();
        assert_eq!(lib.load(c.id).unwrap(), c);
        assert_eq!(count(&lib, "characters"), 1);
        assert_eq!(count(&lib, "items"), 0);
    }

    #[test]
    fn library_ops() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = Library::open(tmp.path().join("data").join("charsheet.db")).unwrap();

        let c = populated();
        lib.save(&c).unwrap();
        lib.create("anna").unwrap();
        let names: Vec<_> = lib.list().unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["anna", "Zed"]);
        let zed = lib.list().unwrap().pop().unwrap();
        assert_eq!(zed, Summary::from(&c));

        let copy = lib.duplicate(c.id).unwrap();
        assert_ne!(copy.id, c.id);
        assert_eq!(copy.name, "Zed (copy)");
        assert_eq!(copy.attacks, c.attacks);

        let exported = tmp.path().join("zed.json");
        lib.export(c.id, &exported).unwrap();
        let imported = lib.import(&exported).unwrap();
        assert_ne!(imported.id, c.id, "colliding id gets replaced");
        assert_eq!(imported.attacks, c.attacks);

        lib.add_roll(
            c.id,
            &StoredRoll::new("x", dice::roll("1d4", RollMode::Normal).unwrap()),
        )
        .unwrap();
        lib.delete(c.id).unwrap();
        assert!(matches!(lib.load(c.id), Err(StorageError::NotFound(_))));
        assert!(matches!(lib.delete(c.id), Err(StorageError::NotFound(_))));
        assert_eq!(lib.list().unwrap().len(), 3);
        assert_eq!(count(&lib, "rolls"), 0, "deleting cascades to the roll log");
        assert_eq!(count(&lib, "character_classes"), 5);
    }

    #[test]
    fn roll_log_is_capped_and_persisted() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("charsheet.db");
        let lib = Library::open(&path).unwrap();
        let c = lib.create("Roller").unwrap();
        let mut last_id = 0;
        for i in 0..MAX_ROLLS + 5 {
            let result = dice::roll("1d20+3", RollMode::Advantage).unwrap();
            last_id = lib
                .add_roll(c.id, &StoredRoll::new(format!("roll {i}"), result))
                .unwrap();
        }
        drop(lib);

        let lib = Library::open(&path).unwrap();
        let rolls = lib.recent_rolls(c.id).unwrap();
        assert_eq!(rolls.len(), MAX_ROLLS);
        assert_eq!(rolls[0].label, format!("roll {}", MAX_ROLLS + 4));
        assert_eq!(rolls[0].id, last_id);
        assert!(rolls.windows(2).all(|w| w[0].id > w[1].id), "newest first");
        assert_eq!(rolls[MAX_ROLLS - 1].label, "roll 5");
        assert_eq!(rolls[0].result.mode, RollMode::Advantage);
        assert_eq!(rolls[0].breakdown, rolls[0].result.breakdown());
        assert!(rolls[0].rolled_at.ends_with('Z'));
        assert_eq!(lib.rolled_at(last_id).unwrap(), rolls[0].rolled_at);

        lib.clear_rolls(c.id).unwrap();
        assert!(lib.recent_rolls(c.id).unwrap().is_empty());
    }

    #[test]
    fn migrates_a_version_1_database() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("charsheet.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(MIGRATIONS[0]).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
            conn.execute_batch(
                "INSERT INTO characters VALUES ('0a5e0000-0000-4000-8000-000000000001', 'Old',
                 '', '', '', '', 0, 10, 7, 0, 0, 0, 0, 0, 30, NULL, 0, 0, 0, 0, 0, 0, 0, 5, 0,
                 'Common', '', '', '', '', '2024-01-01', '2024-01-01');
                 INSERT INTO items VALUES ('0a5e0000-0000-4000-8000-000000000001', 0, 'Rope',
                 1, 10.0, 0, 0, '');
                 INSERT INTO features VALUES ('0a5e0000-0000-4000-8000-000000000001', 0, 'Rage',
                 '', '', 2, 1, 'long');
                 INSERT INTO spells VALUES ('0a5e0000-0000-4000-8000-000000000001', 0, 'Light',
                 0, 1, 0, 0, '');",
            )
            .unwrap();
        }
        let lib = Library::open(&path).unwrap();
        let c = lib
            .load(Uuid::parse_str("0a5e0000-0000-4000-8000-000000000001").unwrap())
            .unwrap();
        assert_eq!(c.hp.current, 7);
        assert_eq!(c.inventory[0].kind, ItemKind::Gear);
        assert!(c.inventory[0].actions.is_empty());
        assert_eq!(c.features[0].uses_spent, 1);
        assert_eq!(c.spells[0].casting_time, Activation::Action);
        assert_eq!(c.spells[0].roll_formula, "");
        assert_eq!(c.spells[0].damage_type, "");
        assert_eq!(c.features[0].uses_formula, "");
        assert_eq!(c.inventory[0].regain, "");
        assert!(c.effects.is_empty());
        lib.save(&c).unwrap();
        assert_eq!(lib.load(c.id).unwrap(), c);
    }

    #[test]
    fn imports_sable() {
        let lib = Library::open_in_memory().unwrap();
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sable.json");
        let sable = lib.import(&path).unwrap();
        assert_eq!(lib.load(sable.id).unwrap(), sable);
    }

    #[test]
    fn revisions_and_data_version() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("charsheet.db");
        let app = Library::open(&path).unwrap();
        let other = Library::open(&path).unwrap();
        let mut c = app.create("Rev").unwrap();
        assert_eq!(app.revision(c.id).unwrap(), 0);
        c.name = "Rev 2".into();
        assert_eq!(app.save(&c).unwrap(), 1);

        let seen = app.data_version().unwrap();
        app.add_roll(
            c.id,
            &StoredRoll::new("x", dice::roll("1d4", RollMode::Normal).unwrap()),
        )
        .unwrap();
        assert_eq!(app.data_version().unwrap(), seen, "own writes don't count");
        assert_eq!(app.revision(c.id).unwrap(), 2);

        other.clear_rolls(c.id).unwrap();
        assert_ne!(app.data_version().unwrap(), seen);
        assert_eq!(app.revision(c.id).unwrap(), 3);
        assert!(matches!(
            app.revision(Uuid::new_v4()),
            Err(StorageError::NotFound(_))
        ));
    }

    #[test]
    fn rejects_newer_database() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("charsheet.db");
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", 99)
            .unwrap();
        assert!(matches!(
            Library::open(&path),
            Err(StorageError::DatabaseTooNew(99))
        ));
    }

    #[test]
    fn rejects_newer_and_garbage_files() {
        let tmp = tempfile::tempdir().unwrap();
        let newer = tmp.path().join("new.json");
        fs::write(&newer, r#"{"schema_version": 99, "name": "Future"}"#).unwrap();
        assert!(matches!(read_file(&newer), Err(StorageError::TooNew(99))));

        let junk = tmp.path().join("junk.json");
        fs::write(&junk, "not json").unwrap();
        assert!(matches!(read_file(&junk), Err(StorageError::Format(_))));

        let lib = Library::open_in_memory().unwrap();
        assert!(lib.import(&junk).is_err());
        assert!(lib.list().unwrap().is_empty());
    }

    #[test]
    fn sparse_files_fill_in_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("old.json");
        fs::write(
            &path,
            r#"{"name": "Minimal", "hp": {"max": 12, "current": 50}}"#,
        )
        .unwrap();
        let c = read_file(&path).unwrap();
        assert_eq!(c.name, "Minimal");
        assert_eq!(c.hp.current, 12);
        assert_eq!(c.speed, 30);
    }
}
