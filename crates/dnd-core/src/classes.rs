//! Class data from the SRD 5.2 (D&D 2024 rules), CC-BY-4.0.

use crate::abilities::Ability;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClassId {
    Barbarian,
    Bard,
    Cleric,
    Druid,
    Fighter,
    Monk,
    Paladin,
    Ranger,
    Rogue,
    Sorcerer,
    Warlock,
    Wizard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CasterKind {
    None,
    Full,
    Half,
    /// Subclass casters such as Eldritch Knight or Arcane Trickster.
    Third,
    Pact,
}

pub struct ClassInfo {
    pub name: &'static str,
    pub hit_die: u8,
    pub saves: [Ability; 2],
    pub caster: CasterKind,
    pub spell_ability: Option<Ability>,
}

impl ClassId {
    pub const ALL: [ClassId; 12] = [
        ClassId::Barbarian,
        ClassId::Bard,
        ClassId::Cleric,
        ClassId::Druid,
        ClassId::Fighter,
        ClassId::Monk,
        ClassId::Paladin,
        ClassId::Ranger,
        ClassId::Rogue,
        ClassId::Sorcerer,
        ClassId::Warlock,
        ClassId::Wizard,
    ];

    pub fn info(self) -> ClassInfo {
        use Ability::*;
        use CasterKind::*;
        let (name, hit_die, saves, caster, spell_ability) = match self {
            ClassId::Barbarian => ("Barbarian", 12, [Str, Con], None, Option::None),
            ClassId::Bard => ("Bard", 8, [Dex, Cha], Full, Some(Cha)),
            ClassId::Cleric => ("Cleric", 8, [Wis, Cha], Full, Some(Wis)),
            ClassId::Druid => ("Druid", 8, [Int, Wis], Full, Some(Wis)),
            ClassId::Fighter => ("Fighter", 10, [Str, Con], None, Option::None),
            ClassId::Monk => ("Monk", 8, [Str, Dex], None, Option::None),
            ClassId::Paladin => ("Paladin", 10, [Wis, Cha], Half, Some(Cha)),
            ClassId::Ranger => ("Ranger", 10, [Str, Dex], Half, Some(Wis)),
            ClassId::Rogue => ("Rogue", 8, [Dex, Int], None, Option::None),
            ClassId::Sorcerer => ("Sorcerer", 6, [Con, Cha], Full, Some(Cha)),
            ClassId::Warlock => ("Warlock", 8, [Wis, Cha], Pact, Some(Cha)),
            ClassId::Wizard => ("Wizard", 6, [Int, Wis], Full, Some(Int)),
        };
        ClassInfo {
            name,
            hit_die,
            saves,
            caster,
            spell_ability,
        }
    }
}

/// Spell slots for levels 1-9 on the Full Caster table, indexed by caster level 1-20.
const FULL_CASTER_SLOTS: [[u8; 9]; 20] = [
    [2, 0, 0, 0, 0, 0, 0, 0, 0],
    [3, 0, 0, 0, 0, 0, 0, 0, 0],
    [4, 2, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 2, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 1, 0, 0, 0, 0, 0],
    [4, 3, 3, 2, 0, 0, 0, 0, 0],
    [4, 3, 3, 3, 1, 0, 0, 0, 0],
    [4, 3, 3, 3, 2, 0, 0, 0, 0],
    [4, 3, 3, 3, 2, 1, 0, 0, 0],
    [4, 3, 3, 3, 2, 1, 0, 0, 0],
    [4, 3, 3, 3, 2, 1, 1, 0, 0],
    [4, 3, 3, 3, 2, 1, 1, 0, 0],
    [4, 3, 3, 3, 2, 1, 1, 1, 0],
    [4, 3, 3, 3, 2, 1, 1, 1, 0],
    [4, 3, 3, 3, 2, 1, 1, 1, 1],
    [4, 3, 3, 3, 3, 1, 1, 1, 1],
    [4, 3, 3, 3, 3, 2, 1, 1, 1],
    [4, 3, 3, 3, 3, 2, 2, 1, 1],
];

/// Spell slots for a caster level (0 means no slots).
pub fn slots_for_caster_level(caster_level: u32) -> [u8; 9] {
    match caster_level {
        0 => [0; 9],
        n => FULL_CASTER_SLOTS[(n.min(20) - 1) as usize],
    }
}

/// Spellcasting class levels as `(class, level, override)`, where `override` turns a
/// non-casting class into a third caster (Eldritch Knight, Arcane Trickster).
///
/// One class uses its own table: half casters use the full table at ceil(level/2),
/// which matches the 2024 Paladin/Ranger table exactly, and third casters start at level 3.
/// Multiclassing adds full levels, half levels (rounded up, 2024 rule) and third levels
/// (rounded down).
pub fn spell_slots(classes: &[(ClassId, u32, bool)]) -> [u8; 9] {
    let kinds: Vec<(CasterKind, u32)> = classes
        .iter()
        .map(|&(class, level, third)| {
            let kind = match class.info().caster {
                CasterKind::None if third => CasterKind::Third,
                kind => kind,
            };
            (kind, level)
        })
        .filter(|(kind, _)| !matches!(kind, CasterKind::None | CasterKind::Pact))
        .collect();

    let caster_level = match kinds.as_slice() {
        [] => 0,
        [(CasterKind::Third, level)] if *level < 3 => 0,
        [(CasterKind::Third, level)] => level.div_ceil(3),
        _ => kinds
            .iter()
            .map(|&(kind, level)| match kind {
                CasterKind::Full => level,
                CasterKind::Half => level.div_ceil(2),
                CasterKind::Third => level / 3,
                CasterKind::None | CasterKind::Pact => 0,
            })
            .sum(),
    };
    slots_for_caster_level(caster_level)
}

/// Warlock Pact Magic: (number of slots, slot level) at a Warlock level.
pub fn pact_slots(warlock_level: u32) -> (u8, u8) {
    match warlock_level {
        0 => (0, 0),
        1 => (1, 1),
        2 => (2, 1),
        3..=4 => (2, 2),
        5..=6 => (2, 3),
        7..=8 => (2, 4),
        9..=10 => (2, 5),
        11..=16 => (3, 5),
        _ => (4, 5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_full_caster() {
        assert_eq!(
            spell_slots(&[(ClassId::Wizard, 1, false)]),
            [2, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            spell_slots(&[(ClassId::Wizard, 5, false)]),
            [4, 3, 2, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            spell_slots(&[(ClassId::Cleric, 20, false)]),
            [4, 3, 3, 3, 3, 2, 2, 1, 1]
        );
    }

    #[test]
    fn single_half_caster_matches_2024_table() {
        let expect = |level, slots: [u8; 9]| {
            assert_eq!(
                spell_slots(&[(ClassId::Paladin, level, false)]),
                slots,
                "paladin {level}"
            )
        };
        expect(1, [2, 0, 0, 0, 0, 0, 0, 0, 0]);
        expect(2, [2, 0, 0, 0, 0, 0, 0, 0, 0]);
        expect(3, [3, 0, 0, 0, 0, 0, 0, 0, 0]);
        expect(5, [4, 2, 0, 0, 0, 0, 0, 0, 0]);
        expect(9, [4, 3, 2, 0, 0, 0, 0, 0, 0]);
        expect(13, [4, 3, 3, 1, 0, 0, 0, 0, 0]);
        expect(17, [4, 3, 3, 3, 1, 0, 0, 0, 0]);
        expect(20, [4, 3, 3, 3, 2, 0, 0, 0, 0]);
    }

    #[test]
    fn third_caster() {
        assert_eq!(spell_slots(&[(ClassId::Fighter, 2, true)]), [0; 9]);
        assert_eq!(
            spell_slots(&[(ClassId::Fighter, 3, true)]),
            [2, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            spell_slots(&[(ClassId::Rogue, 7, true)]),
            [4, 2, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(spell_slots(&[(ClassId::Fighter, 3, false)]), [0; 9]);
    }

    #[test]
    fn multiclass() {
        // Paladin 3 / Sorcerer 2: ceil(3/2) + 2 = 4.
        assert_eq!(
            spell_slots(&[(ClassId::Paladin, 3, false), (ClassId::Sorcerer, 2, false)]),
            [4, 3, 0, 0, 0, 0, 0, 0, 0]
        );
        // Warlock levels don't count toward the multiclass table.
        assert_eq!(
            spell_slots(&[(ClassId::Warlock, 5, false), (ClassId::Wizard, 3, false)]),
            [4, 2, 0, 0, 0, 0, 0, 0, 0]
        );
        // Fighter 5 / Wizard 1 (no Eldritch Knight): only the wizard level counts.
        assert_eq!(
            spell_slots(&[(ClassId::Fighter, 5, false), (ClassId::Wizard, 1, false)]),
            [2, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn pact_magic() {
        assert_eq!(pact_slots(1), (1, 1));
        assert_eq!(pact_slots(5), (2, 3));
        assert_eq!(pact_slots(11), (3, 5));
        assert_eq!(pact_slots(20), (4, 5));
    }
}
