use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ability {
    Str,
    Dex,
    Con,
    Int,
    Wis,
    Cha,
}

impl Ability {
    pub const ALL: [Ability; 6] = [
        Ability::Str,
        Ability::Dex,
        Ability::Con,
        Ability::Int,
        Ability::Wis,
        Ability::Cha,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Ability::Str => "Strength",
            Ability::Dex => "Dexterity",
            Ability::Con => "Constitution",
            Ability::Int => "Intelligence",
            Ability::Wis => "Wisdom",
            Ability::Cha => "Charisma",
        }
    }

    pub fn abbrev(self) -> &'static str {
        match self {
            Ability::Str => "STR",
            Ability::Dex => "DEX",
            Ability::Con => "CON",
            Ability::Int => "INT",
            Ability::Wis => "WIS",
            Ability::Cha => "CHA",
        }
    }
}

/// Ability modifier for a score: floor((score - 10) / 2).
pub fn modifier(score: i32) -> i32 {
    (score - 10).div_euclid(2)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Skill {
    Acrobatics,
    AnimalHandling,
    Arcana,
    Athletics,
    Deception,
    History,
    Insight,
    Intimidation,
    Investigation,
    Medicine,
    Nature,
    Perception,
    Performance,
    Persuasion,
    Religion,
    SleightOfHand,
    Stealth,
    Survival,
}

impl Skill {
    pub const ALL: [Skill; 18] = [
        Skill::Acrobatics,
        Skill::AnimalHandling,
        Skill::Arcana,
        Skill::Athletics,
        Skill::Deception,
        Skill::History,
        Skill::Insight,
        Skill::Intimidation,
        Skill::Investigation,
        Skill::Medicine,
        Skill::Nature,
        Skill::Perception,
        Skill::Performance,
        Skill::Persuasion,
        Skill::Religion,
        Skill::SleightOfHand,
        Skill::Stealth,
        Skill::Survival,
    ];

    pub fn ability(self) -> Ability {
        use Skill::*;
        match self {
            Athletics => Ability::Str,
            Acrobatics | SleightOfHand | Stealth => Ability::Dex,
            Arcana | History | Investigation | Nature | Religion => Ability::Int,
            AnimalHandling | Insight | Medicine | Perception | Survival => Ability::Wis,
            Deception | Intimidation | Performance | Persuasion => Ability::Cha,
        }
    }

    pub fn name(self) -> &'static str {
        use Skill::*;
        match self {
            Acrobatics => "Acrobatics",
            AnimalHandling => "Animal Handling",
            Arcana => "Arcana",
            Athletics => "Athletics",
            Deception => "Deception",
            History => "History",
            Insight => "Insight",
            Intimidation => "Intimidation",
            Investigation => "Investigation",
            Medicine => "Medicine",
            Nature => "Nature",
            Perception => "Perception",
            Performance => "Performance",
            Persuasion => "Persuasion",
            Religion => "Religion",
            SleightOfHand => "Sleight of Hand",
            Stealth => "Stealth",
            Survival => "Survival",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Proficiency {
    #[default]
    None,
    Proficient,
    Expertise,
}

impl Proficiency {
    /// Bonus contributed by this proficiency level. `half_prof` covers Jack of All Trades.
    pub fn bonus(self, prof_bonus: i32, half_prof: bool) -> i32 {
        match self {
            Proficiency::None if half_prof => prof_bonus / 2,
            Proficiency::None => 0,
            Proficiency::Proficient => prof_bonus,
            Proficiency::Expertise => prof_bonus * 2,
        }
    }
}

/// Proficiency bonus by total character level (2 at 1-4, up to 6 at 17-20).
pub fn proficiency_bonus(total_level: u32) -> i32 {
    let level = total_level.clamp(1, 20) as i32;
    2 + (level - 1) / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers() {
        for (score, expected) in [
            (1, -5),
            (3, -4),
            (8, -1),
            (9, -1),
            (10, 0),
            (11, 0),
            (12, 1),
            (15, 2),
            (20, 5),
            (30, 10),
        ] {
            assert_eq!(modifier(score), expected, "score {score}");
        }
    }

    #[test]
    fn proficiency_by_level() {
        let expected = [2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6];
        for (i, pb) in expected.iter().enumerate() {
            assert_eq!(proficiency_bonus(i as u32 + 1), *pb, "level {}", i + 1);
        }
        assert_eq!(proficiency_bonus(0), 2);
    }

    #[test]
    fn jack_of_all_trades_rounds_down() {
        assert_eq!(Proficiency::None.bonus(3, true), 1);
        assert_eq!(Proficiency::Expertise.bonus(3, true), 6);
    }
}
