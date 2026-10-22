//! One JSON file per character, `<uuid>.json`, in the library directory.

use crate::character::{Character, SCHEMA_VERSION};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("not a valid character file: {0}")]
    Format(#[from] serde_json::Error),
    #[error("this character was saved by a newer version of the app (format {0})")]
    TooNew(u32),
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

pub struct Library {
    dir: PathBuf,
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
    /// The default library at `$XDG_DATA_HOME/charsheet/characters`.
    pub fn open_default() -> Result<Library, StorageError> {
        let dir = dirs::data_dir()
            .ok_or(StorageError::NoDataDir)?
            .join("charsheet")
            .join("characters");
        Library::open(dir)
    }

    pub fn open(dir: impl Into<PathBuf>) -> Result<Library, StorageError> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        Ok(Library { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, id: Uuid) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// Every readable character, sorted by name. Files that can't be read are skipped.
    pub fn list(&self) -> Result<Vec<Summary>, StorageError> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "json") {
                match read_file(&path) {
                    Ok(c) => out.push(Summary::from(&c)),
                    Err(e) => eprintln!("skipping {}: {e}", path.display()),
                }
            }
        }
        out.sort_by_key(|s| s.name.to_lowercase());
        Ok(out)
    }

    pub fn load(&self, id: Uuid) -> Result<Character, StorageError> {
        read_file(&self.path(id))
    }

    pub fn save(&self, character: &Character) -> Result<(), StorageError> {
        write_file(&self.path(character.id), character)
    }

    pub fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        fs::remove_file(self.path(id))?;
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
        if self.path(character.id).exists() {
            character.id = Uuid::new_v4();
        }
        self.save(&character)?;
        Ok(character)
    }

    pub fn export(&self, id: Uuid, path: &Path) -> Result<(), StorageError> {
        write_file(path, &self.load(id)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{Proficiency, Skill};
    use crate::character::Attack;

    #[test]
    fn round_trip_and_library_ops() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = Library::open(tmp.path().join("chars")).unwrap();

        let mut c = lib.create("Zed").unwrap();
        c.skills.insert(Skill::Stealth, Proficiency::Expertise);
        c.attacks.push(Attack {
            name: "Rapier".into(),
            ..Default::default()
        });
        lib.save(&c).unwrap();
        assert_eq!(lib.load(c.id).unwrap(), c);

        lib.create("anna").unwrap();
        let names: Vec<_> = lib.list().unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["anna", "Zed"]);

        let copy = lib.duplicate(c.id).unwrap();
        assert_ne!(copy.id, c.id);
        assert_eq!(copy.name, "Zed (copy)");

        let exported = tmp.path().join("zed.json");
        lib.export(c.id, &exported).unwrap();
        let imported = lib.import(&exported).unwrap();
        assert_ne!(imported.id, c.id, "colliding id gets replaced");
        assert_eq!(imported.attacks, c.attacks);

        lib.delete(c.id).unwrap();
        assert_eq!(lib.list().unwrap().len(), 3);
        assert!(
            !tmp.path().join("chars").read_dir().unwrap().any(|e| e
                .unwrap()
                .path()
                .to_string_lossy()
                .ends_with(".tmp"))
        );
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

        let lib = Library::open(tmp.path()).unwrap();
        assert!(
            lib.list().unwrap().is_empty(),
            "unreadable files are skipped"
        );
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
