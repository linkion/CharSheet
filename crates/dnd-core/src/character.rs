use crate::abilities::{Ability, Proficiency, Skill, modifier, proficiency_bonus};
use crate::armor::ArmorKind;
use crate::classes::{self, CasterKind, ClassId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClassLevel {
    pub class: ClassId,
    pub subclass: String,
    pub level: u32,
    pub hit_dice_spent: u32,
    /// Grants third-caster spellcasting (Eldritch Knight, Arcane Trickster).
    pub third_caster: bool,
}

impl Default for ClassLevel {
    fn default() -> Self {
        ClassLevel {
            class: ClassId::Fighter,
            subclass: String::new(),
            level: 1,
            hit_dice_spent: 0,
            third_caster: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HitPoints {
    pub max: i32,
    pub current: i32,
    pub temp: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DeathSaves {
    pub successes: u8,
    pub failures: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Attack {
    pub name: String,
    pub ability: Ability,
    pub proficient: bool,
    /// Damage dice without the ability modifier, e.g. `1d8`.
    pub damage: String,
    pub damage_type: String,
    /// Extra bonus to hit and damage (magic weapons).
    pub bonus: i32,
    pub add_ability_to_damage: bool,
    /// 2024 Weapon Mastery property, e.g. Sap, Topple, Vex.
    pub mastery: String,
    pub notes: String,
}

impl Default for Attack {
    fn default() -> Self {
        Attack {
            name: "New attack".into(),
            ability: Ability::Str,
            proficient: true,
            damage: "1d6".into(),
            damage_type: String::new(),
            bonus: 0,
            add_ability_to_damage: true,
            mastery: String::new(),
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Spell {
    pub name: String,
    /// 0 for cantrips.
    pub level: u8,
    pub prepared: bool,
    pub ritual: bool,
    pub concentration: bool,
    pub notes: String,
}

impl Default for Spell {
    fn default() -> Self {
        Spell {
            name: "New spell".into(),
            level: 0,
            prepared: false,
            ritual: false,
            concentration: false,
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Currency {
    pub cp: i64,
    pub sp: i64,
    pub ep: i64,
    pub gp: i64,
    pub pp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Item {
    pub name: String,
    pub quantity: u32,
    /// Weight of one item in pounds.
    pub weight: f64,
    pub requires_attunement: bool,
    pub attuned: bool,
    pub notes: String,
}

impl Default for Item {
    fn default() -> Self {
        Item {
            name: "New item".into(),
            quantity: 1,
            weight: 0.0,
            requires_attunement: false,
            attuned: false,
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Recharge {
    #[default]
    None,
    Short,
    Long,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Feature {
    pub name: String,
    pub source: String,
    pub description: String,
    pub uses_max: u32,
    pub uses_spent: u32,
    pub recharge: Recharge,
}

impl Default for Feature {
    fn default() -> Self {
        Feature {
            name: "New feature".into(),
            source: String::new(),
            description: String::new(),
            uses_max: 0,
            uses_spent: 0,
            recharge: Recharge::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Condition {
    Blinded,
    Charmed,
    Deafened,
    Frightened,
    Grappled,
    Incapacitated,
    Invisible,
    Paralyzed,
    Petrified,
    Poisoned,
    Prone,
    Restrained,
    Stunned,
    Unconscious,
}

impl Condition {
    pub const ALL: [Condition; 14] = [
        Condition::Blinded,
        Condition::Charmed,
        Condition::Deafened,
        Condition::Frightened,
        Condition::Grappled,
        Condition::Incapacitated,
        Condition::Invisible,
        Condition::Paralyzed,
        Condition::Petrified,
        Condition::Poisoned,
        Condition::Prone,
        Condition::Restrained,
        Condition::Stunned,
        Condition::Unconscious,
    ];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Character {
    pub schema_version: u32,
    pub id: Uuid,
    pub name: String,
    pub player: String,
    pub species: String,
    pub background: String,
    pub alignment: String,
    pub classes: Vec<ClassLevel>,
    /// Base scores before background and other bonuses, in STR..CHA order.
    pub base_scores: [i32; 6],
    /// Background (+2/+1 or +1/+1/+1), feats and other bonuses, in STR..CHA order.
    pub score_bonuses: [i32; 6],
    /// Save proficiencies on top of the starting class's.
    pub extra_saves: BTreeSet<Ability>,
    pub skills: BTreeMap<Skill, Proficiency>,
    pub jack_of_all_trades: bool,
    pub hp: HitPoints,
    pub death_saves: DeathSaves,
    pub heroic_inspiration: bool,
    /// 0-6; each level gives -2 to d20 tests and -5 ft speed (2024 rules).
    pub exhaustion: u8,
    pub conditions: BTreeSet<Condition>,
    pub speed: i32,
    pub armor: Option<ArmorKind>,
    pub shield: bool,
    pub ac_bonus: i32,
    pub initiative_bonus: i32,
    pub attacks: Vec<Attack>,
    pub spells: Vec<Spell>,
    pub spell_slots_used: [u8; 9],
    pub pact_slots_used: u8,
    pub currency: Currency,
    pub inventory: Vec<Item>,
    pub features: Vec<Feature>,
    pub languages: String,
    pub other_proficiencies: String,
    pub appearance: String,
    pub backstory: String,
    pub notes: String,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            schema_version: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            name: "New Adventurer".into(),
            player: String::new(),
            species: String::new(),
            background: String::new(),
            alignment: String::new(),
            classes: vec![ClassLevel::default()],
            base_scores: [10; 6],
            score_bonuses: [0; 6],
            extra_saves: BTreeSet::new(),
            skills: BTreeMap::new(),
            jack_of_all_trades: false,
            hp: HitPoints {
                max: 10,
                current: 10,
                temp: 0,
            },
            death_saves: DeathSaves::default(),
            heroic_inspiration: false,
            exhaustion: 0,
            conditions: BTreeSet::new(),
            speed: 30,
            armor: None,
            shield: false,
            ac_bonus: 0,
            initiative_bonus: 0,
            attacks: Vec::new(),
            spells: Vec::new(),
            spell_slots_used: [0; 9],
            pact_slots_used: 0,
            currency: Currency::default(),
            inventory: Vec::new(),
            features: Vec::new(),
            languages: "Common".into(),
            other_proficiencies: String::new(),
            appearance: String::new(),
            backstory: String::new(),
            notes: String::new(),
        }
    }
}

impl Character {
    pub fn new(name: &str) -> Character {
        Character {
            name: name.to_owned(),
            ..Default::default()
        }
    }

    pub fn score(&self, ability: Ability) -> i32 {
        let i = ability.index();
        self.base_scores[i] + self.score_bonuses[i]
    }

    pub fn ability_mod(&self, ability: Ability) -> i32 {
        modifier(self.score(ability))
    }

    pub fn total_level(&self) -> u32 {
        self.classes.iter().map(|c| c.level).sum::<u32>().max(1)
    }

    pub fn proficiency_bonus(&self) -> i32 {
        proficiency_bonus(self.total_level())
    }

    /// Penalty applied to every d20 test from Exhaustion.
    pub fn d20_penalty(&self) -> i32 {
        -2 * self.exhaustion as i32
    }

    pub fn class_level(&self, class: ClassId) -> u32 {
        self.classes
            .iter()
            .filter(|c| c.class == class)
            .map(|c| c.level)
            .sum()
    }

    pub fn save_proficient(&self, ability: Ability) -> bool {
        // Multiclassing does not grant the new class's saves, so only the first class counts.
        let from_class = self
            .classes
            .first()
            .is_some_and(|c| c.class.info().saves.contains(&ability));
        from_class || self.extra_saves.contains(&ability)
    }

    pub fn save_mod(&self, ability: Ability) -> i32 {
        let prof = if self.save_proficient(ability) {
            self.proficiency_bonus()
        } else {
            0
        };
        self.ability_mod(ability) + prof + self.d20_penalty()
    }

    pub fn skill_proficiency(&self, skill: Skill) -> Proficiency {
        self.skills.get(&skill).copied().unwrap_or_default()
    }

    pub fn skill_mod(&self, skill: Skill) -> i32 {
        let prof = self
            .skill_proficiency(skill)
            .bonus(self.proficiency_bonus(), self.jack_of_all_trades);
        self.ability_mod(skill.ability()) + prof + self.d20_penalty()
    }

    /// Passive score: 10 + skill modifier (Exhaustion only affects actual d20 rolls).
    pub fn passive(&self, skill: Skill) -> i32 {
        10 + self.skill_mod(skill) - self.d20_penalty()
    }

    pub fn initiative(&self) -> i32 {
        let joat = if self.jack_of_all_trades {
            self.proficiency_bonus() / 2
        } else {
            0
        };
        self.ability_mod(Ability::Dex) + self.initiative_bonus + joat + self.d20_penalty()
    }

    pub fn speed(&self) -> i32 {
        (self.speed - 5 * self.exhaustion as i32).max(0)
    }

    pub fn armor_class(&self) -> i32 {
        let dex = self.ability_mod(Ability::Dex);
        let shield = if self.shield { 2 } else { 0 };
        let base = match self.armor {
            Some(armor) => armor.ac(dex),
            None => {
                let mut best = 10 + dex;
                if self.class_level(ClassId::Barbarian) > 0 {
                    best = best.max(10 + dex + self.ability_mod(Ability::Con));
                }
                // Monk Unarmored Defense requires no shield.
                if self.class_level(ClassId::Monk) > 0 && !self.shield {
                    best = best.max(10 + dex + self.ability_mod(Ability::Wis));
                }
                best
            }
        };
        base + shield + self.ac_bonus
    }

    pub fn spell_slots_max(&self) -> [u8; 9] {
        let classes: Vec<_> = self
            .classes
            .iter()
            .map(|c| (c.class, c.level, c.third_caster))
            .collect();
        classes::spell_slots(&classes)
    }

    pub fn pact_slots(&self) -> (u8, u8) {
        classes::pact_slots(self.class_level(ClassId::Warlock))
    }

    /// Distinct spellcasting abilities across classes, in class order.
    pub fn spellcasting_abilities(&self) -> Vec<(String, Ability)> {
        let mut seen = Vec::new();
        for c in &self.classes {
            let info = c.class.info();
            let ability = match (
                info.spell_ability,
                c.third_caster && info.caster == CasterKind::None,
            ) {
                (Some(a), _) => a,
                (None, true) => Ability::Int,
                (None, false) => continue,
            };
            if !seen.iter().any(|(_, a)| *a == ability) {
                seen.push((info.name.to_owned(), ability));
            }
        }
        seen
    }

    pub fn spell_save_dc(&self, ability: Ability) -> i32 {
        8 + self.proficiency_bonus() + self.ability_mod(ability)
    }

    pub fn spell_attack(&self, ability: Ability) -> i32 {
        self.proficiency_bonus() + self.ability_mod(ability) + self.d20_penalty()
    }

    pub fn attack_bonus(&self, attack: &Attack) -> i32 {
        let prof = if attack.proficient {
            self.proficiency_bonus()
        } else {
            0
        };
        self.ability_mod(attack.ability) + prof + attack.bonus + self.d20_penalty()
    }

    /// Full damage expression, e.g. `1d8+3`.
    pub fn damage_expression(&self, attack: &Attack) -> String {
        let flat = attack.bonus
            + if attack.add_ability_to_damage {
                self.ability_mod(attack.ability)
            } else {
                0
            };
        match flat {
            0 => attack.damage.clone(),
            n if n > 0 => format!("{}+{n}", attack.damage),
            n => format!("{}{n}", attack.damage),
        }
    }

    /// Carrying capacity in pounds (Medium creature: STR x 15).
    pub fn carrying_capacity(&self) -> f64 {
        self.score(Ability::Str) as f64 * 15.0
    }

    pub fn carried_weight(&self) -> f64 {
        self.inventory
            .iter()
            .map(|i| i.weight * i.quantity as f64)
            .sum()
    }

    pub fn attuned_count(&self) -> usize {
        self.inventory.iter().filter(|i| i.attuned).count()
    }

    pub fn class_summary(&self) -> String {
        self.classes
            .iter()
            .map(|c| {
                let name = c.class.info().name;
                if c.subclass.is_empty() {
                    format!("{name} {}", c.level)
                } else {
                    format!("{name} ({}) {}", c.subclass, c.level)
                }
            })
            .collect::<Vec<_>>()
            .join(" / ")
    }

    // --- Hit points -----------------------------------------------------------------

    /// Takes damage, using up temporary HP first.
    pub fn take_damage(&mut self, amount: i32) {
        let amount = amount.max(0);
        let from_temp = amount.min(self.hp.temp);
        self.hp.temp -= from_temp;
        self.hp.current = (self.hp.current - (amount - from_temp)).max(0);
    }

    pub fn heal(&mut self, amount: i32) {
        if amount <= 0 {
            return;
        }
        if self.hp.current == 0 {
            self.death_saves = DeathSaves::default();
        }
        self.hp.current = (self.hp.current + amount).min(self.hp.max);
    }

    /// Temporary HP don't stack; keep whichever is higher.
    pub fn set_temp_hp(&mut self, amount: i32) {
        self.hp.temp = self.hp.temp.max(amount.max(0));
    }

    // --- Rests ----------------------------------------------------------------------

    /// Spends one hit die of the class at `class_index`, healing by the roll plus the
    /// CON modifier (minimum 1). Returns the roll, or None if no dice are left.
    pub fn spend_hit_die(
        &mut self,
        class_index: usize,
        rng: &mut impl rand::Rng,
    ) -> Option<(u32, i32)> {
        let con = self.ability_mod(Ability::Con);
        let class = self.classes.get_mut(class_index)?;
        if class.hit_dice_spent >= class.level {
            return None;
        }
        class.hit_dice_spent += 1;
        let roll = rng.random_range(1..=class.class.info().hit_die as u32);
        let healed = (roll as i32 + con).max(1);
        self.heal(healed);
        Some((roll, healed))
    }

    pub fn short_rest(&mut self) {
        self.pact_slots_used = 0;
        for f in &mut self.features {
            if f.recharge == Recharge::Short {
                f.uses_spent = 0;
            }
        }
    }

    pub fn long_rest(&mut self) {
        self.short_rest();
        self.hp.current = self.hp.max;
        self.hp.temp = 0;
        self.death_saves = DeathSaves::default();
        self.spell_slots_used = [0; 9];
        for c in &mut self.classes {
            c.hit_dice_spent = 0;
        }
        for f in &mut self.features {
            if f.recharge == Recharge::Long {
                f.uses_spent = 0;
            }
        }
        self.exhaustion = self.exhaustion.saturating_sub(1);
    }

    /// Clamps values that edits could push out of range.
    pub fn normalize(&mut self) {
        for c in &mut self.classes {
            c.level = c.level.clamp(1, 20);
            c.hit_dice_spent = c.hit_dice_spent.min(c.level);
        }
        for s in self.base_scores.iter_mut() {
            *s = (*s).clamp(1, 30);
        }
        self.hp.max = self.hp.max.max(1);
        self.hp.current = self.hp.current.clamp(0, self.hp.max);
        self.hp.temp = self.hp.temp.max(0);
        self.death_saves.successes = self.death_saves.successes.min(3);
        self.death_saves.failures = self.death_saves.failures.min(3);
        self.exhaustion = self.exhaustion.min(6);
        let slots = self.spell_slots_max();
        for (used, max) in self.spell_slots_used.iter_mut().zip(slots) {
            *used = (*used).min(max);
        }
        self.pact_slots_used = self.pact_slots_used.min(self.pact_slots().0);
        for spell in &mut self.spells {
            spell.level = spell.level.min(9);
        }
        for f in &mut self.features {
            f.uses_spent = f.uses_spent.min(f.uses_max);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// Level 5 Fighter / 1 Wizard, STR 16 DEX 14 CON 15 INT 13 WIS 10 CHA 8.
    fn fighter_wizard() -> Character {
        Character {
            classes: vec![
                ClassLevel {
                    class: ClassId::Fighter,
                    level: 5,
                    ..Default::default()
                },
                ClassLevel {
                    class: ClassId::Wizard,
                    level: 1,
                    ..Default::default()
                },
            ],
            base_scores: [15, 13, 14, 13, 10, 8],
            score_bonuses: [1, 1, 1, 0, 0, 0],
            skills: BTreeMap::from([
                (Skill::Athletics, Proficiency::Proficient),
                (Skill::Perception, Proficiency::Expertise),
            ]),
            ..Character::new("Test")
        }
    }

    #[test]
    fn derived_stats() {
        let c = fighter_wizard();
        assert_eq!(c.total_level(), 6);
        assert_eq!(c.proficiency_bonus(), 3);
        assert_eq!(c.ability_mod(Ability::Str), 3);
        assert_eq!(c.save_mod(Ability::Str), 6);
        assert_eq!(c.save_mod(Ability::Con), 5);
        // Wizard was taken second, so no INT save proficiency.
        assert_eq!(c.save_mod(Ability::Int), 1);
        assert_eq!(c.skill_mod(Skill::Athletics), 6);
        assert_eq!(c.skill_mod(Skill::Perception), 6);
        assert_eq!(c.passive(Skill::Perception), 16);
        assert_eq!(c.initiative(), 2);
        assert_eq!(c.armor_class(), 12);
        assert_eq!(c.spell_slots_max(), [2, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            c.spellcasting_abilities(),
            vec![("Wizard".to_owned(), Ability::Int)]
        );
        assert_eq!(c.spell_save_dc(Ability::Int), 12);
        assert_eq!(c.spell_attack(Ability::Int), 4);
        assert_eq!(c.class_summary(), "Fighter 5 / Wizard 1");
    }

    #[test]
    fn armor_class_variants() {
        let mut c = fighter_wizard();
        c.armor = Some(ArmorKind::ChainMail);
        c.shield = true;
        assert_eq!(c.armor_class(), 18);

        let mut barb = Character {
            classes: vec![ClassLevel {
                class: ClassId::Barbarian,
                level: 1,
                ..Default::default()
            }],
            base_scores: [16, 14, 16, 8, 10, 10],
            ..Default::default()
        };
        assert_eq!(barb.armor_class(), 15);
        barb.shield = true;
        assert_eq!(barb.armor_class(), 17);

        let mut monk = Character {
            classes: vec![ClassLevel {
                class: ClassId::Monk,
                level: 1,
                ..Default::default()
            }],
            base_scores: [10, 16, 12, 10, 16, 8],
            ..Default::default()
        };
        assert_eq!(monk.armor_class(), 16);
        monk.shield = true;
        assert_eq!(monk.armor_class(), 15);
    }

    #[test]
    fn exhaustion_penalties() {
        let mut c = fighter_wizard();
        c.exhaustion = 2;
        assert_eq!(c.skill_mod(Skill::Athletics), 2);
        assert_eq!(c.passive(Skill::Perception), 16);
        assert_eq!(c.speed(), 20);
        assert_eq!(c.spell_save_dc(Ability::Int), 12);
    }

    #[test]
    fn jack_of_all_trades() {
        let mut c = fighter_wizard();
        c.jack_of_all_trades = true;
        assert_eq!(c.skill_mod(Skill::Stealth), 3);
        assert_eq!(c.skill_mod(Skill::Athletics), 6);
        assert_eq!(c.initiative(), 3);
    }

    #[test]
    fn damage_uses_temp_hp_first() {
        let mut c = fighter_wizard();
        c.hp = HitPoints {
            max: 40,
            current: 30,
            temp: 5,
        };
        c.take_damage(8);
        assert_eq!(
            c.hp,
            HitPoints {
                max: 40,
                current: 27,
                temp: 0
            }
        );
        c.take_damage(100);
        assert_eq!(c.hp.current, 0);
        c.death_saves.failures = 2;
        c.heal(50);
        assert_eq!(c.hp.current, 40);
        assert_eq!(c.death_saves, DeathSaves::default());
        c.set_temp_hp(6);
        c.set_temp_hp(3);
        assert_eq!(c.hp.temp, 6);
    }

    #[test]
    fn rests() {
        let mut c = fighter_wizard();
        c.classes.push(ClassLevel {
            class: ClassId::Warlock,
            level: 2,
            ..Default::default()
        });
        c.hp = HitPoints {
            max: 50,
            current: 10,
            temp: 3,
        };
        c.spell_slots_used = [2, 1, 0, 0, 0, 0, 0, 0, 0];
        c.pact_slots_used = 2;
        c.exhaustion = 2;
        c.features = vec![
            Feature {
                name: "Second Wind".into(),
                uses_max: 2,
                uses_spent: 2,
                recharge: Recharge::Short,
                ..Default::default()
            },
            Feature {
                name: "Arcane Recovery".into(),
                uses_max: 1,
                uses_spent: 1,
                recharge: Recharge::Long,
                ..Default::default()
            },
        ];

        let mut rng = StdRng::seed_from_u64(1);
        let (roll, healed) = c.spend_hit_die(0, &mut rng).unwrap();
        assert!((1..=10).contains(&roll));
        assert_eq!(healed, roll as i32 + 2);
        assert_eq!(c.hp.current, 10 + healed);
        assert_eq!(c.classes[0].hit_dice_spent, 1);
        c.classes[1].hit_dice_spent = 1;
        assert_eq!(c.spend_hit_die(1, &mut rng), None);

        c.short_rest();
        assert_eq!(c.pact_slots_used, 0);
        assert_eq!(c.features[0].uses_spent, 0);
        assert_eq!(c.features[1].uses_spent, 1);
        assert_eq!(c.spell_slots_used[0], 2);

        c.long_rest();
        assert_eq!(
            c.hp,
            HitPoints {
                max: 50,
                current: 50,
                temp: 0
            }
        );
        assert_eq!(c.spell_slots_used, [0; 9]);
        assert_eq!(c.features[1].uses_spent, 0);
        assert!(c.classes.iter().all(|c| c.hit_dice_spent == 0));
        assert_eq!(c.exhaustion, 1);
    }

    #[test]
    fn attacks() {
        let c = fighter_wizard();
        let sword = Attack {
            name: "Longsword".into(),
            damage: "1d8".into(),
            bonus: 1,
            ..Default::default()
        };
        assert_eq!(c.attack_bonus(&sword), 7);
        assert_eq!(c.damage_expression(&sword), "1d8+4");
        let weak = Attack {
            ability: Ability::Cha,
            damage: "1d4".into(),
            ..Default::default()
        };
        assert_eq!(c.damage_expression(&weak), "1d4-1");
    }

    #[test]
    fn normalize_clamps() {
        let mut c = fighter_wizard();
        c.hp = HitPoints {
            max: 20,
            current: 99,
            temp: -3,
        };
        c.spell_slots_used = [9; 9];
        c.exhaustion = 12;
        c.classes[0].level = 40;
        c.normalize();
        assert_eq!(
            c.hp,
            HitPoints {
                max: 20,
                current: 20,
                temp: 0
            }
        );
        assert_eq!(c.spell_slots_used, [2, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(c.exhaustion, 6);
        assert_eq!(c.classes[0].level, 20);
    }
}
