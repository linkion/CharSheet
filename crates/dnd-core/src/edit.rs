//! Generic edits addressed by JSON Pointer (RFC 6901), e.g. `/hp/current` or `/attacks/0/name`.
//!
//! Each edit goes through serde, so a value of the wrong type is rejected and the
//! character is left unchanged.

use crate::character::Character;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("no field at {0}")]
    NoSuchPath(String),
    #[error("{0} is not a list")]
    NotAList(String),
    #[error("index {1} is out of range for {0}")]
    OutOfRange(String, usize),
    #[error("invalid value: {0}")]
    Invalid(#[from] serde_json::Error),
}

fn apply(
    c: &mut Character,
    f: impl FnOnce(&mut Value) -> Result<(), EditError>,
) -> Result<(), EditError> {
    let mut value = serde_json::to_value(&*c)?;
    f(&mut value)?;
    let mut updated: Character = serde_json::from_value(value)?;
    updated.id = c.id;
    updated.normalize();
    *c = updated;
    Ok(())
}

fn list<'a>(value: &'a mut Value, pointer: &str) -> Result<&'a mut Vec<Value>, EditError> {
    value
        .pointer_mut(pointer)
        .ok_or_else(|| EditError::NoSuchPath(pointer.into()))?
        .as_array_mut()
        .ok_or_else(|| EditError::NotAList(pointer.into()))
}

/// Replaces the value at `pointer`. A missing key in an object (like a skill entry) is inserted.
pub fn set(c: &mut Character, pointer: &str, new_value: Value) -> Result<(), EditError> {
    apply(c, |value| {
        if let Some(slot) = value.pointer_mut(pointer) {
            *slot = new_value;
            return Ok(());
        }
        let (parent, key) = pointer
            .rsplit_once('/')
            .ok_or_else(|| EditError::NoSuchPath(pointer.into()))?;
        let key = key.replace("~1", "/").replace("~0", "~");
        let object = value
            .pointer_mut(parent)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| EditError::NoSuchPath(pointer.into()))?;
        object.insert(key, new_value);
        Ok(())
    })
}

/// Appends an item to the list at `pointer`. `item` may be partial; missing fields use defaults.
pub fn push(c: &mut Character, pointer: &str, item: Value) -> Result<(), EditError> {
    apply(c, |value| {
        list(value, pointer)?.push(item);
        Ok(())
    })
}

pub fn remove(c: &mut Character, pointer: &str, index: usize) -> Result<(), EditError> {
    apply(c, |value| {
        let items = list(value, pointer)?;
        if index >= items.len() {
            return Err(EditError::OutOfRange(pointer.into(), index));
        }
        items.remove(index);
        Ok(())
    })
}

pub fn move_item(
    c: &mut Character,
    pointer: &str,
    from: usize,
    to: usize,
) -> Result<(), EditError> {
    apply(c, |value| {
        let items = list(value, pointer)?;
        if from >= items.len() || to >= items.len() {
            return Err(EditError::OutOfRange(pointer.into(), from.max(to)));
        }
        let item = items.remove(from);
        items.insert(to, item);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{Proficiency, Skill};
    use crate::armor::ArmorKind;
    use serde_json::json;

    #[test]
    fn set_scalar_and_nested() {
        let mut c = Character::new("A");
        set(&mut c, "/name", json!("Brakka")).unwrap();
        set(&mut c, "/hp/max", json!(30)).unwrap();
        set(&mut c, "/base_scores/0", json!(17)).unwrap();
        set(&mut c, "/armor", json!("half_plate")).unwrap();
        assert_eq!(c.name, "Brakka");
        assert_eq!(c.hp.max, 30);
        assert_eq!(c.base_scores[0], 17);
        assert_eq!(c.armor, Some(ArmorKind::HalfPlate));
        set(&mut c, "/armor", Value::Null).unwrap();
        assert_eq!(c.armor, None);
    }

    #[test]
    fn set_inserts_missing_map_keys() {
        let mut c = Character::new("A");
        set(&mut c, "/skills/sleight_of_hand", json!("expertise")).unwrap();
        assert_eq!(
            c.skill_proficiency(Skill::SleightOfHand),
            Proficiency::Expertise
        );
    }

    #[test]
    fn invalid_edits_leave_character_untouched() {
        let mut c = Character::new("A");
        let before = c.clone();
        assert!(set(&mut c, "/hp/max", json!("lots")).is_err());
        assert!(set(&mut c, "/skills/basket_weaving", json!("proficient")).is_err());
        assert!(set(&mut c, "/nope/deeper", json!(1)).is_err());
        assert!(remove(&mut c, "/attacks", 0).is_err());
        assert!(push(&mut c, "/name", json!({})).is_err());
        assert_eq!(c, before);
    }

    #[test]
    fn edits_cannot_change_id() {
        let mut c = Character::new("A");
        let id = c.id;
        assert!(set(&mut c, "/id", json!(uuid::Uuid::new_v4())).is_ok());
        assert_eq!(c.id, id);
    }

    #[test]
    fn list_operations() {
        let mut c = Character::new("A");
        push(
            &mut c,
            "/attacks",
            json!({ "name": "Dagger", "damage": "1d4" }),
        )
        .unwrap();
        push(&mut c, "/attacks", json!({})).unwrap();
        assert_eq!(c.attacks.len(), 2);
        assert_eq!(c.attacks[0].name, "Dagger");
        assert!(c.attacks[0].proficient);
        assert_eq!(c.attacks[1].name, "New attack");
        move_item(&mut c, "/attacks", 1, 0).unwrap();
        assert_eq!(c.attacks[1].name, "Dagger");
        set(&mut c, "/attacks/1/mastery", json!("Nick")).unwrap();
        assert_eq!(c.attacks[1].mastery, "Nick");
        remove(&mut c, "/attacks", 0).unwrap();
        assert_eq!(c.attacks.len(), 1);
    }

    #[test]
    fn edits_are_normalized() {
        let mut c = Character::new("A");
        set(&mut c, "/hp/current", json!(500)).unwrap();
        assert_eq!(c.hp.current, c.hp.max);
    }
}
