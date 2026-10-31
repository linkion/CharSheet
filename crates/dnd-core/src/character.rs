use crate::abilities::{Ability, Proficiency, Skill, modifier, proficiency_bonus};
use crate::armor::ArmorKind;
use crate::classes::{self, CasterKind, ClassId};
use crate::dice::{self, RollMode, RollResult};
use crate::effects::{
    Action, Activation, Cost, Effect, Modifier, ModifierKind, Outcome, RollEffect,
};
use crate::formula::{self, FormulaError, Vars};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

/// 2 added items with weapon stats, modifiers and actions, plus active effects. 3 added
/// formulas that keep values following the character, and dawn recharge.
pub const SCHEMA_VERSION: u32 = 3;

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
    pub casting_time: Activation,
    /// Casts that cost no slot, recharging on a long rest (Favored Enemy).
    pub free_uses_max: u32,
    pub free_uses_spent: u32,
    /// Dice rolled when cast, e.g. `2d8+2`; empty for none.
    pub roll: String,
    /// Keeps `roll` following the character, e.g. `{cantrip}d10`; empty for fixed dice.
    pub roll_formula: String,
    pub roll_effect: RollEffect,
    /// Applied to the character when cast.
    pub effect: Option<Effect>,
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
            casting_time: Activation::Action,
            free_uses_max: 0,
            free_uses_spent: 0,
            roll: String::new(),
            roll_formula: String::new(),
            roll_effect: RollEffect::None,
            effect: None,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    #[default]
    Gear,
    Weapon,
    Armor,
    Shield,
    Ammunition,
    Consumable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WeaponStats {
    /// Damage dice without modifiers, e.g. `1d8`.
    pub damage: String,
    pub damage_type: String,
    pub finesse: bool,
    /// A ranged weapon (bow, firearm), as opposed to a melee weapon that can be thrown.
    pub ranged: bool,
    pub thrown: bool,
    pub light: bool,
    /// e.g. `60/240`.
    pub range: String,
    pub mastery: String,
    /// Bonus to attack and damage rolls.
    pub magic_bonus: i32,
    pub proficient: bool,
    /// Shots before reloading (the Reload property); 0 if it never needs reloading.
    pub capacity: u32,
    pub loaded: u32,
    /// Name of the ammunition item fired and used up; empty for untracked ammunition.
    pub ammo: String,
    /// Damage dice when used with two hands (the Versatile property), e.g. `1d10`.
    pub versatile: String,
    /// The properties as printed, e.g. `Versatile (1d10)`. Empty on weapons entered by hand,
    /// which show the ones the fields above record.
    pub properties: Vec<String>,
}

impl WeaponStats {
    /// The weapon's properties as printed.
    pub fn property_labels(&self) -> Vec<String> {
        if !self.properties.is_empty() {
            return self.properties.clone();
        }
        let range = |name: &str| match self.range.as_str() {
            "" => name.to_owned(),
            range => format!("{name} ({range})"),
        };
        let mut labels = Vec::new();
        if self.finesse {
            labels.push("Finesse".to_owned());
        }
        if self.light {
            labels.push("Light".to_owned());
        }
        if self.thrown {
            labels.push(range("Thrown"));
        }
        if !self.versatile.is_empty() {
            labels.push(format!("Versatile ({})", self.versatile));
        }
        labels
    }
}

impl Default for WeaponStats {
    fn default() -> Self {
        WeaponStats {
            damage: "1d6".into(),
            damage_type: String::new(),
            finesse: false,
            ranged: false,
            thrown: false,
            light: false,
            range: String::new(),
            mastery: String::new(),
            magic_bonus: 0,
            proficient: true,
            capacity: 0,
            loaded: 0,
            ammo: String::new(),
            versatile: String::new(),
            properties: Vec::new(),
        }
    }
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
    pub kind: ItemKind,
    pub equipped: bool,
    pub weapon: Option<WeaponStats>,
    pub armor: Option<ArmorKind>,
    /// Magic bonus to AC for armor and shields.
    pub ac_bonus: i32,
    pub charges_max: u32,
    pub charges_spent: u32,
    pub recharge: Recharge,
    /// Dice of charges regained at dawn, e.g. `1d6+1`; empty regains them all.
    pub regain: String,
    /// Apply while equipped (and attuned, if it needs attunement). Ammunition modifiers
    /// apply to the weapon firing it instead.
    pub modifiers: Vec<Modifier>,
    pub actions: Vec<Action>,
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
            kind: ItemKind::Gear,
            equipped: false,
            weapon: None,
            armor: None,
            ac_bonus: 0,
            charges_max: 0,
            charges_spent: 0,
            recharge: Recharge::None,
            regain: String::new(),
            modifiers: Vec::new(),
            actions: Vec::new(),
        }
    }
}

impl Item {
    /// Whether the item's modifiers currently apply.
    pub fn active(&self) -> bool {
        self.equipped && (!self.requires_attunement || self.attuned) && self.quantity > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Recharge {
    #[default]
    None,
    Short,
    Long,
    /// Regains `regain` dice of charges at dawn, which is taken as the end of a long rest.
    Dawn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Feature {
    pub name: String,
    pub source: String,
    pub description: String,
    pub uses_max: u32,
    /// Keeps `uses_max` following the character, e.g. `{pb}`; empty for a fixed number.
    pub uses_formula: String,
    pub uses_spent: u32,
    pub recharge: Recharge,
    /// Always apply.
    pub modifiers: Vec<Modifier>,
    pub actions: Vec<Action>,
}

impl Default for Feature {
    fn default() -> Self {
        Feature {
            name: "New feature".into(),
            source: String::new(),
            description: String::new(),
            uses_max: 0,
            uses_formula: String::new(),
            uses_spent: 0,
            recharge: Recharge::None,
            modifiers: Vec::new(),
            actions: Vec::new(),
        }
    }
}

/// A damage rider on a weapon attack, such as Hunter's Mark or Slash Draw.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rider {
    /// Unique per character; also what `riders_used` records.
    pub name: String,
    pub dice: String,
    pub damage_type: String,
    pub once_per_turn: bool,
    pub used: bool,
}

/// A modifier that can be switched on and off, with where to find it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Toggle {
    /// JSON Pointer of the modifier, e.g. `/inventory/2/modifiers/0`.
    pub pointer: String,
    pub source: String,
    /// The modifier's label, or its description when it has none.
    pub label: String,
    pub description: String,
    pub enabled: bool,
}

impl Toggle {
    fn new(source: &str, pointer: String, m: &Modifier) -> Toggle {
        let description = m.describe();
        Toggle {
            pointer,
            source: source.to_owned(),
            label: if m.text.is_empty() {
                description.clone()
            } else {
                m.text.clone()
            },
            description,
            enabled: m.enabled,
        }
    }
}

/// An attack with an equipped weapon, with every modifier worked in.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WeaponAttack {
    pub item: usize,
    pub name: String,
    pub thrown: bool,
    pub ranged: bool,
    pub ability: Ability,
    pub to_hit: i32,
    /// Base damage with flat bonuses, e.g. `1d8+4`; riders are listed separately.
    pub damage: String,
    pub damage_type: String,
    pub range: String,
    pub mastery: String,
    /// The weapon's properties as printed, e.g. `Versatile (1d10)`.
    pub properties: Vec<String>,
    pub riders: Vec<Rider>,
    /// Modifiers on or off for this attack that can be switched, such as a Fire mode.
    pub toggles: Vec<Toggle>,
    /// Notes from the loaded ammunition and note modifiers on the weapon.
    pub notes: Vec<String>,
    pub capacity: u32,
    pub loaded: u32,
    pub ammo: String,
    /// How many of the tracked ammunition are left, if it's tracked.
    pub ammo_left: Option<u32>,
}

/// The ways of using one weapon that get an attack of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grip {
    Normal,
    Thrown,
    TwoHanded,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlayError {
    #[error("no such {0}")]
    NotFound(&'static str),
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Dice(#[from] crate::dice::DiceError),
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
    /// Effects currently on the character.
    pub effects: Vec<Effect>,
    /// Combat round, 0 outside combat.
    pub round: u32,
    /// Once-per-turn riders already used this turn.
    pub riders_used: BTreeSet<String>,
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
            effects: Vec::new(),
            round: 0,
            riders_used: BTreeSet::new(),
        }
    }
}

fn resolve_modifiers(modifiers: &mut [Modifier], vars: &Vars) -> Result<(), FormulaError> {
    modifiers
        .iter_mut()
        .filter(|m| !m.value_formula.trim().is_empty())
        .map(|m| formula::number(&m.value_formula, vars).map(|n| m.value = n))
        .fold(Ok(()), Result::and)
}

fn resolve_actions(actions: &mut [Action], vars: &Vars) -> Result<(), FormulaError> {
    actions
        .iter_mut()
        .map(|a| {
            let roll = if a.roll_formula.trim().is_empty() {
                Ok(())
            } else {
                formula::dice(&a.roll_formula, vars).map(|d| a.roll = d)
            };
            let effect = match &mut a.effect {
                Some(e) => resolve_modifiers(&mut e.modifiers, vars),
                None => Ok(()),
            };
            roll.and(effect)
        })
        .fold(Ok(()), Result::and)
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
        let bonus: i32 = self
            .modifiers_of(ModifierKind::Save)
            .filter(|m| m.ability.is_none_or(|a| a == ability))
            .map(|m| m.value)
            .sum();
        self.ability_mod(ability) + prof + bonus + self.d20_penalty()
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
        let bonus = self.modifier_total(ModifierKind::Initiative);
        self.ability_mod(Ability::Dex) + self.initiative_bonus + bonus + joat + self.d20_penalty()
    }

    pub fn speed(&self) -> i32 {
        let bonus = self.modifier_total(ModifierKind::Speed);
        (self.speed + bonus - 5 * self.exhaustion as i32).max(0)
    }

    pub fn armor_class(&self) -> i32 {
        let dex = self.ability_mod(Ability::Dex);
        let worn = self
            .inventory
            .iter()
            .find(|i| i.kind == ItemKind::Armor && i.armor.is_some() && i.active());
        let shield_item = self
            .inventory
            .iter()
            .find(|i| i.kind == ItemKind::Shield && i.active());
        let has_shield = self.shield || shield_item.is_some();
        let shield = if has_shield {
            2 + shield_item.map_or(0, |i| i.ac_bonus)
        } else {
            0
        };
        let base = match (worn, self.armor) {
            (Some(item), _) => item.armor.map_or(10 + dex, |a| a.ac(dex)) + item.ac_bonus,
            (None, Some(armor)) => armor.ac(dex),
            (None, None) => {
                let mut best = 10 + dex;
                if self.class_level(ClassId::Barbarian) > 0 {
                    best = best.max(10 + dex + self.ability_mod(Ability::Con));
                }
                // Monk Unarmored Defense requires no shield.
                if self.class_level(ClassId::Monk) > 0 && !has_shield {
                    best = best.max(10 + dex + self.ability_mod(Ability::Wis));
                }
                best
            }
        };
        base + shield + self.ac_bonus + self.modifier_total(ModifierKind::ArmorClass)
    }

    // --- Modifiers and effects --------------------------------------------------------

    /// Every modifier from features, equipped items and active effects, switched on or not,
    /// with the name of what it comes from and its JSON Pointer. Ammunition is left out; it
    /// only affects the weapon firing it.
    fn modifier_sources(&self) -> Vec<(&str, String, &Modifier)> {
        fn each<'a>(
            list: &'static str,
            entries: impl Iterator<Item = (usize, &'a str, &'a [Modifier])>,
        ) -> impl Iterator<Item = (&'a str, String, &'a Modifier)> {
            entries.flat_map(move |(i, source, modifiers)| {
                modifiers
                    .iter()
                    .enumerate()
                    .map(move |(j, m)| (source, format!("/{list}/{i}/modifiers/{j}"), m))
            })
        }
        let features = each(
            "features",
            self.features
                .iter()
                .enumerate()
                .map(|(i, f)| (i, f.name.as_str(), f.modifiers.as_slice())),
        );
        let items = each(
            "inventory",
            self.inventory
                .iter()
                .enumerate()
                .filter(|(_, i)| i.active() && i.kind != ItemKind::Ammunition)
                .map(|(i, it)| (i, it.name.as_str(), it.modifiers.as_slice())),
        );
        let effects = each(
            "effects",
            self.effects
                .iter()
                .enumerate()
                .map(|(i, e)| (i, e.name.as_str(), e.modifiers.as_slice())),
        );
        features.chain(items).chain(effects).collect()
    }

    /// Every modifier in force, with the name of what it comes from: features, equipped
    /// items and active effects, leaving out any switched off.
    pub fn modifiers(&self) -> Vec<(&str, &Modifier)> {
        self.modifier_sources()
            .into_iter()
            .filter(|(_, _, m)| m.enabled)
            .map(|(source, _, m)| (source, m))
            .collect()
    }

    /// Every modifier that can be switched on and off, outside ammunition.
    pub fn toggles(&self) -> Vec<Toggle> {
        self.modifier_sources()
            .into_iter()
            .filter(|(_, _, m)| m.toggleable)
            .map(|(source, pointer, m)| Toggle::new(source, pointer, m))
            .collect()
    }

    fn modifiers_of(&self, kind: ModifierKind) -> impl Iterator<Item = &Modifier> {
        self.modifiers()
            .into_iter()
            .map(|(_, m)| m)
            .filter(move |m| m.kind == kind)
    }

    fn modifier_total(&self, kind: ModifierKind) -> i32 {
        self.modifiers_of(kind).map(|m| m.value).sum()
    }

    /// Hit point maximum including effects such as Aid.
    pub fn max_hp(&self) -> i32 {
        (self.hp.max + self.modifier_total(ModifierKind::MaxHp)).max(1)
    }

    /// Damage types resisted, without duplicates, in the order they were found.
    pub fn resistances(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for m in self.modifiers_of(ModifierKind::Resistance) {
            let t = m.damage_type.trim().to_lowercase();
            if !t.is_empty() && !out.contains(&t) {
                out.push(t);
            }
        }
        out
    }

    /// Note modifiers in force, as `(source, text)`.
    pub fn notes_in_force(&self) -> Vec<(String, String)> {
        self.modifiers()
            .into_iter()
            .filter(|(_, m)| m.kind == ModifierKind::Note && !m.text.is_empty())
            .map(|(source, m)| (source.to_owned(), m.text.clone()))
            .collect()
    }

    pub fn concentrating_on(&self) -> Option<&Effect> {
        self.effects.iter().find(|e| e.concentration)
    }

    /// Adds an effect. Concentrating on a new one ends the old; reapplying one by the same
    /// name replaces it.
    pub fn apply_effect(&mut self, effect: Effect) {
        self.effects
            .retain(|e| e.name != effect.name && !(effect.concentration && e.concentration));
        self.effects.push(effect);
        self.clamp_hp();
    }

    pub fn end_effect(&mut self, index: usize) -> Result<Effect, PlayError> {
        if index >= self.effects.len() {
            return Err(PlayError::NotFound("effect"));
        }
        let effect = self.effects.remove(index);
        self.clamp_hp();
        Ok(effect)
    }

    /// Starts the character's next turn: once-per-turn riders reset and effects with a
    /// round limit count down, ending at zero.
    pub fn new_turn(&mut self) {
        self.riders_used.clear();
        self.round += 1;
        // An effect at 0 lasts until removed, so only ones counting down can end.
        self.effects.retain_mut(|e| {
            if e.rounds == 0 {
                return true;
            }
            e.rounds -= 1;
            e.rounds > 0
        });
        self.clamp_hp();
    }

    /// Leaves combat: the round counter and riders reset and effects with a round limit end.
    pub fn end_combat(&mut self) {
        self.riders_used.clear();
        self.round = 0;
        self.effects.retain(|e| e.rounds == 0);
        self.clamp_hp();
    }

    fn clamp_hp(&mut self) {
        self.hp.current = self.hp.current.min(self.max_hp());
    }

    // --- Weapons ----------------------------------------------------------------------

    fn ammo_item(&self, weapon: &WeaponStats) -> Option<&Item> {
        if weapon.ammo.is_empty() {
            return None;
        }
        self.inventory
            .iter()
            .find(|i| i.name == weapon.ammo && i.kind == ItemKind::Ammunition)
    }

    /// One attack per equipped weapon, plus a thrown attack for melee weapons that can be thrown.
    pub fn weapon_attacks(&self) -> Vec<WeaponAttack> {
        let mut out = Vec::new();
        for (index, item) in self.inventory.iter().enumerate() {
            let Some(weapon) = item.weapon.as_ref().filter(|_| item.active()) else {
                continue;
            };
            out.push(self.weapon_attack(index, item, weapon, Grip::Normal));
            if weapon.thrown && !weapon.ranged {
                out.push(self.weapon_attack(index, item, weapon, Grip::Thrown));
            }
            if !weapon.versatile.trim().is_empty() {
                out.push(self.weapon_attack(index, item, weapon, Grip::TwoHanded));
            }
        }
        out
    }

    fn weapon_attack(
        &self,
        index: usize,
        item: &Item,
        weapon: &WeaponStats,
        grip: Grip,
    ) -> WeaponAttack {
        let thrown = grip == Grip::Thrown;
        let ranged_attack = weapon.ranged || thrown;
        let (str_mod, dex_mod) = (
            self.ability_mod(Ability::Str),
            self.ability_mod(Ability::Dex),
        );
        let ability = if weapon.finesse {
            if dex_mod >= str_mod {
                Ability::Dex
            } else {
                Ability::Str
            }
        } else if weapon.ranged {
            Ability::Dex
        } else {
            Ability::Str
        };
        let ammo = if weapon.ranged {
            self.ammo_item(weapon)
        } else {
            None
        };
        let ammo_index = ammo.and_then(|a| self.inventory.iter().position(|i| std::ptr::eq(i, a)));

        // Ammunition modifiers apply whatever their scope; the item's own always do too.
        let own = item.modifiers.iter().enumerate().map(|(j, m)| {
            (
                item.name.as_str(),
                format!("/inventory/{index}/modifiers/{j}"),
                m,
                true,
            )
        });
        let from_ammo = ammo.into_iter().zip(ammo_index).flat_map(|(a, ai)| {
            a.modifiers.iter().enumerate().map(move |(j, m)| {
                (
                    a.name.as_str(),
                    format!("/inventory/{ai}/modifiers/{j}"),
                    m,
                    true,
                )
            })
        });
        // Attack and damage modifiers on another weapon (its Fire mode, a +1) stay on it.
        let others = self
            .modifier_sources()
            .into_iter()
            .filter(|(source, _, m)| {
                *source != item.name
                    && !(matches!(m.kind, ModifierKind::AttackRoll | ModifierKind::Damage)
                        && self
                            .inventory
                            .iter()
                            .any(|i| i.name == *source && i.weapon.is_some()))
            })
            .map(|(source, pointer, m)| (source, pointer, m, false));
        let candidates: Vec<(&str, String, &Modifier)> = own
            .chain(from_ammo)
            .chain(others)
            .filter(|(_, _, m, always)| *always || m.scope.matches(ranged_attack, weapon.ranged))
            .map(|(source, pointer, m, _)| (source, pointer, m))
            .collect();
        let toggles: Vec<Toggle> = candidates
            .iter()
            .filter(|(_, _, m)| {
                m.toggleable && matches!(m.kind, ModifierKind::AttackRoll | ModifierKind::Damage)
            })
            .map(|(source, pointer, m)| Toggle::new(source, pointer.clone(), m))
            .collect();
        let applicable: Vec<(&str, &Modifier)> = candidates
            .into_iter()
            .filter(|(_, _, m)| m.enabled)
            .map(|(source, _, m)| (source, m))
            .collect();

        let prof = if weapon.proficient {
            self.proficiency_bonus()
        } else {
            0
        };
        let attack_bonus: i32 = applicable
            .iter()
            .filter(|(_, m)| m.kind == ModifierKind::AttackRoll)
            .map(|(_, m)| m.value)
            .sum();
        let to_hit = self.ability_mod(ability)
            + prof
            + weapon.magic_bonus
            + attack_bonus
            + self.d20_penalty();

        let mut flat = self.ability_mod(ability) + weapon.magic_bonus;
        let mut riders = Vec::new();
        let mut notes = Vec::new();
        for (source, m) in &applicable {
            match m.kind {
                ModifierKind::Damage if m.dice.trim().is_empty() => flat += m.value,
                ModifierKind::Damage => {
                    let name = if m.text.is_empty() {
                        source.to_string()
                    } else {
                        format!("{source} ({})", m.text)
                    };
                    riders.push(Rider {
                        used: m.once_per_turn && self.riders_used.contains(&name),
                        name,
                        dice: m.dice.trim().to_owned(),
                        damage_type: m.damage_type.clone(),
                        once_per_turn: m.once_per_turn,
                    });
                }
                ModifierKind::Note
                    if !m.text.is_empty()
                        && (*source == item.name || ammo.is_some_and(|a| a.name == *source)) =>
                {
                    notes.push(m.text.clone())
                }
                _ => {}
            }
        }
        let dice = match grip {
            Grip::TwoHanded => &weapon.versatile,
            _ => &weapon.damage,
        };
        let damage = match flat {
            0 => dice.clone(),
            n if n > 0 => format!("{dice}+{n}"),
            n => format!("{dice}{n}"),
        };
        let range = if ranged_attack {
            weapon.range.clone()
        } else {
            "melee".to_owned()
        };
        WeaponAttack {
            item: index,
            name: match grip {
                Grip::Normal => item.name.clone(),
                Grip::Thrown => format!("{} (thrown)", item.name),
                Grip::TwoHanded => format!("{} (two-handed)", item.name),
            },
            thrown,
            ranged: ranged_attack,
            ability,
            to_hit,
            damage,
            damage_type: weapon.damage_type.clone(),
            range,
            mastery: weapon.mastery.clone(),
            properties: weapon.property_labels(),
            riders,
            toggles,
            notes,
            capacity: weapon.capacity,
            loaded: weapon.loaded,
            ammo: ammo.map(|a| a.name.clone()).unwrap_or_default(),
            ammo_left: ammo.map(|a| a.quantity),
        }
    }

    /// Spends one shot: a round from the cylinder and, if tracked, one of the ammunition.
    pub fn fire(&mut self, item: usize) -> Result<(), PlayError> {
        let weapon = self
            .inventory
            .get(item)
            .and_then(|i| i.weapon.clone())
            .ok_or(PlayError::NotFound("weapon"))?;
        if weapon.capacity > 0 && weapon.loaded == 0 {
            return Err(PlayError::Unavailable(format!(
                "{} is empty; reload it",
                self.inventory[item].name
            )));
        }
        if let Some(ammo) = self.ammo_item(&weapon) {
            if ammo.quantity == 0 {
                return Err(PlayError::Unavailable(format!("Out of {}", ammo.name)));
            }
            let name = ammo.name.clone();
            if let Some(a) = self
                .inventory
                .iter_mut()
                .find(|i| i.name == name && i.kind == ItemKind::Ammunition)
            {
                a.quantity -= 1;
            }
        }
        if let Some(w) = self.inventory[item].weapon.as_mut()
            && w.capacity > 0
        {
            w.loaded -= 1;
        }
        Ok(())
    }

    pub fn reload(&mut self, item: usize) -> Result<(), PlayError> {
        let weapon = self
            .inventory
            .get_mut(item)
            .and_then(|i| i.weapon.as_mut())
            .ok_or(PlayError::NotFound("weapon"))?;
        weapon.loaded = weapon.capacity;
        Ok(())
    }

    /// The damage expression for an attack with every rider not in `excluded`, doubled on a
    /// critical hit. Marks the once-per-turn riders it includes as used.
    pub fn weapon_damage(
        &mut self,
        attack: &WeaponAttack,
        crit: bool,
        excluded: &[String],
    ) -> Result<String, PlayError> {
        let mut expression = attack.damage.clone();
        for rider in &attack.riders {
            if rider.used || excluded.contains(&rider.name) {
                continue;
            }
            expression.push('+');
            expression.push_str(&rider.dice);
            if rider.once_per_turn {
                self.riders_used.insert(rider.name.clone());
            }
        }
        if crit {
            expression = crate::dice::double_dice(&expression)?;
        }
        Ok(expression)
    }

    // --- Actions and spells -----------------------------------------------------------

    fn finish_action(&mut self, source: &str, action: Action, spent: String) -> Outcome {
        if let Some(mut effect) = action.effect {
            if effect.source.is_empty() {
                effect.source = source.to_owned();
            }
            self.apply_effect(effect);
        }
        Outcome {
            label: format!("{source}: {}", action.name),
            roll: action.roll,
            roll_effect: action.roll_effect,
            spent,
        }
    }

    /// Uses one of an item's actions, spending its cost and applying its effect.
    pub fn use_item_action(&mut self, item: usize, action: usize) -> Result<Outcome, PlayError> {
        let it = self
            .inventory
            .get(item)
            .ok_or(PlayError::NotFound("item"))?;
        let act = it
            .actions
            .get(action)
            .cloned()
            .ok_or(PlayError::NotFound("action"))?;
        let name = it.name.clone();
        let it = &mut self.inventory[item];
        let n = act.cost_amount;
        let spent = match act.cost {
            Cost::None => String::new(),
            Cost::Quantity => {
                if it.quantity < n {
                    return Err(PlayError::Unavailable(format!("No {name} left")));
                }
                it.quantity -= n;
                format!("{} left", it.quantity)
            }
            Cost::Charges | Cost::Uses => {
                if it.charges_max.saturating_sub(it.charges_spent) < n {
                    return Err(PlayError::Unavailable(format!(
                        "{name} has no charges left"
                    )));
                }
                it.charges_spent += n;
                format!(
                    "{} of {} charges left",
                    it.charges_max - it.charges_spent,
                    it.charges_max
                )
            }
        };
        Ok(self.finish_action(&name, act, spent))
    }

    /// Uses one of a feature's actions, spending its uses and applying its effect.
    pub fn use_feature_action(
        &mut self,
        feature: usize,
        action: usize,
    ) -> Result<Outcome, PlayError> {
        let f = self
            .features
            .get_mut(feature)
            .ok_or(PlayError::NotFound("feature"))?;
        let act = f
            .actions
            .get(action)
            .cloned()
            .ok_or(PlayError::NotFound("action"))?;
        let name = f.name.clone();
        let spent = if act.cost == Cost::None {
            String::new()
        } else {
            if f.uses_max.saturating_sub(f.uses_spent) < act.cost_amount {
                return Err(PlayError::Unavailable(format!("{name} has no uses left")));
            }
            f.uses_spent += act.cost_amount;
            format!("{} of {} uses left", f.uses_max - f.uses_spent, f.uses_max)
        };
        Ok(self.finish_action(&name, act, spent))
    }

    /// Casts a spell with a slot of `slot_level`, or a free use when it's 0. Cantrips cost
    /// nothing. Pact slots are used when no regular slot of that level is left.
    pub fn cast_spell(&mut self, spell: usize, slot_level: u8) -> Result<Outcome, PlayError> {
        let sp = self
            .spells
            .get(spell)
            .ok_or(PlayError::NotFound("spell"))?
            .clone();
        let spent = if sp.level == 0 {
            String::new()
        } else if slot_level == 0 {
            if sp.free_uses_spent >= sp.free_uses_max {
                return Err(PlayError::Unavailable(format!(
                    "No free casts of {} left",
                    sp.name
                )));
            }
            let s = &mut self.spells[spell];
            s.free_uses_spent += 1;
            format!(
                "{} of {} free casts left",
                s.free_uses_max - s.free_uses_spent,
                s.free_uses_max
            )
        } else {
            if slot_level < sp.level || slot_level > 9 {
                return Err(PlayError::Unavailable(format!(
                    "{} needs a slot of level {} or higher",
                    sp.name, sp.level
                )));
            }
            let i = slot_level as usize - 1;
            let max = self.spell_slots_max()[i];
            let (pact_max, pact_level) = self.pact_slots();
            if self.spell_slots_used[i] < max {
                self.spell_slots_used[i] += 1;
                format!(
                    "{} of {max} level {slot_level} slots left",
                    max - self.spell_slots_used[i]
                )
            } else if pact_level == slot_level && self.pact_slots_used < pact_max {
                self.pact_slots_used += 1;
                format!(
                    "{} of {pact_max} pact slots left",
                    pact_max - self.pact_slots_used
                )
            } else {
                return Err(PlayError::Unavailable(format!(
                    "No level {slot_level} slots left"
                )));
            }
        };
        let action = Action {
            name: "Cast".into(),
            activation: sp.casting_time,
            roll: sp.roll.clone(),
            roll_effect: sp.roll_effect,
            effect: sp.effect.clone().map(|mut e| {
                e.concentration |= sp.concentration;
                e
            }),
            ..Default::default()
        };
        let mut outcome = self.finish_action(&sp.name, action, spent);
        outcome.label = sp.name;
        Ok(outcome)
    }

    /// Applies an action's rolled total: healing or temporary hit points.
    pub fn apply_roll(&mut self, effect: RollEffect, total: i32) {
        match effect {
            RollEffect::None => {}
            RollEffect::Heal => self.heal(total),
            RollEffect::TempHp => self.set_temp_hp(total),
        }
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

    /// The spellcasting ability of the highest-level casting class.
    pub fn main_spell_ability(&self) -> Option<Ability> {
        self.classes
            .iter()
            .filter_map(|c| {
                let info = c.class.info();
                let third = c.third_caster && info.caster == CasterKind::None;
                let ability = info.spell_ability.or(third.then_some(Ability::Int))?;
                Some((c.level, ability))
            })
            .fold(None, |best: Option<(u32, Ability)>, next| match best {
                Some((level, _)) if level >= next.0 => best,
                _ => Some(next),
            })
            .map(|(_, ability)| ability)
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
        self.hp.current = (self.hp.current + amount).min(self.max_hp());
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
        for i in &mut self.inventory {
            if i.recharge == Recharge::Short {
                i.charges_spent = 0;
            }
        }
    }

    pub fn long_rest(&mut self) {
        self.short_rest();
        // Eight hours outlasts concentration and almost every spell; Aid can be cast again.
        self.end_combat();
        self.effects.clear();
        self.hp.current = self.max_hp();
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
        for i in &mut self.inventory {
            if i.recharge == Recharge::Long {
                i.charges_spent = 0;
            }
        }
        for s in &mut self.spells {
            s.free_uses_spent = 0;
        }
        self.exhaustion = self.exhaustion.saturating_sub(1);
    }

    /// Regains charges on items that recharge at dawn, returning each item's name, roll and
    /// the charges it now has. Items with no charges spent don't roll.
    pub fn dawn(&mut self, rng: &mut impl Rng) -> Vec<(String, RollResult, u32)> {
        let mut rolled = Vec::new();
        for item in &mut self.inventory {
            if item.recharge != Recharge::Dawn || item.charges_spent == 0 {
                continue;
            }
            if item.regain.trim().is_empty() {
                item.charges_spent = 0;
                continue;
            }
            // Edits reject bad dice, so one here came from a hand-edited import.
            let Ok(roll) = dice::roll_with(&item.regain, RollMode::Normal, rng) else {
                continue;
            };
            item.charges_spent = item
                .charges_spent
                .saturating_sub(roll.total.max(0).unsigned_abs());
            let left = item.charges_max - item.charges_spent;
            rolled.push((item.name.clone(), roll, left));
        }
        rolled
    }

    /// Recomputes every value that has a formula, returning the first bad formula. A bad one
    /// leaves its value alone, so a character with one still loads.
    pub fn resolve_formulas(&mut self) -> Result<(), FormulaError> {
        let vars = Vars::of(self);
        let mut first = Ok(());
        let mut note = |r: Result<(), FormulaError>| {
            if first.is_ok() {
                first = r;
            }
        };
        for f in &mut self.features {
            if !f.uses_formula.trim().is_empty() {
                note(formula::number(&f.uses_formula, &vars).map(|n| f.uses_max = n.max(0) as u32));
            }
            note(resolve_modifiers(&mut f.modifiers, &vars));
            note(resolve_actions(&mut f.actions, &vars));
        }
        for item in &mut self.inventory {
            if !item.regain.trim().is_empty() {
                note(formula::dice(&item.regain, &vars).map(|_| ()));
            }
            note(resolve_modifiers(&mut item.modifiers, &vars));
            note(resolve_actions(&mut item.actions, &vars));
        }
        for s in &mut self.spells {
            if !s.roll_formula.trim().is_empty() {
                note(formula::dice(&s.roll_formula, &vars).map(|d| s.roll = d));
            }
            if let Some(e) = &mut s.effect {
                note(resolve_modifiers(&mut e.modifiers, &vars));
            }
        }
        for e in &mut self.effects {
            note(resolve_modifiers(&mut e.modifiers, &vars));
        }
        first
    }

    /// Clamps values that edits could push out of range, and follows formulas.
    pub fn normalize(&mut self) {
        for c in &mut self.classes {
            c.level = c.level.clamp(1, 20);
            c.hit_dice_spent = c.hit_dice_spent.min(c.level);
        }
        for s in self.base_scores.iter_mut() {
            *s = (*s).clamp(1, 30);
        }
        // First, as maximum hit points and use counts below depend on the results. A bad
        // formula is reported by edits and otherwise leaves the last good value in place.
        let _ = self.resolve_formulas();
        self.hp.max = self.hp.max.max(1);
        self.hp.current = self.hp.current.clamp(0, self.max_hp());
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
            spell.free_uses_spent = spell.free_uses_spent.min(spell.free_uses_max);
        }
        for f in &mut self.features {
            f.uses_spent = f.uses_spent.min(f.uses_max);
        }
        for item in &mut self.inventory {
            item.charges_spent = item.charges_spent.min(item.charges_max);
            // Picking the weapon kind in the editor brings up weapon stats to fill in.
            if item.kind == ItemKind::Weapon && item.weapon.is_none() {
                item.weapon = Some(WeaponStats::default());
            }
            if let Some(w) = item.weapon.as_mut() {
                w.loaded = w.loaded.min(w.capacity);
            }
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

    fn sable() -> Character {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sable.json");
        crate::storage::read_file(&path).expect("examples/sable.json loads")
    }

    fn attack<'a>(attacks: &'a [WeaponAttack], name: &str) -> &'a WeaponAttack {
        attacks.iter().find(|a| a.name == name).unwrap()
    }

    fn item_index(c: &Character, name: &str) -> usize {
        c.inventory.iter().position(|i| i.name == name).unwrap()
    }

    #[test]
    fn sable_example() {
        let c = sable();
        assert_eq!(c.armor_class(), 16);
        assert_eq!(c.initiative(), 7);
        assert_eq!(c.speed(), 40);
        assert_eq!(c.skill_mod(Skill::Perception), 8);
        assert_eq!(c.spell_save_dc(Ability::Wis), 13);
        assert_eq!(c.spell_attack(Ability::Wis), 5);
        assert_eq!(c.spell_slots_max(), [4, 2, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(c.resistances(), ["necrotic"]);

        let attacks = c.weapon_attacks();
        let katana = attack(&attacks, "Katana");
        assert_eq!((katana.to_hit, katana.damage.as_str()), (7, "1d8+4"));
        let riders: Vec<_> = katana.riders.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(riders, ["Slash Draw (melee)"]);

        // Archery applies to the revolvers, not to a thrown dagger; Slash Draw's ranged
        // rider applies to both.
        let revolver = attack(&attacks, "Revolver");
        assert_eq!((revolver.to_hit, revolver.damage.as_str()), (9, "1d6+4"));
        let iron = attack(&attacks, "Black Iron Revolver");
        assert_eq!((iron.to_hit, iron.damage.as_str()), (11, "1d8+6"));
        assert_eq!(iron.ammo_left, Some(60));
        let thrown = attack(&attacks, "Dagger (thrown)");
        assert_eq!((thrown.to_hit, thrown.damage.as_str()), (7, "1d4+4"));
        assert_eq!(thrown.riders[0].name, "Slash Draw (ranged)");
        assert_eq!(
            attack(&attacks, "Dagger").riders[0].name,
            "Slash Draw (melee)"
        );
    }

    #[test]
    fn bleeding_rounds_change_the_shot() {
        let mut c = sable();
        let iron = item_index(&c, "Black Iron Revolver");
        c.inventory[iron].weapon.as_mut().unwrap().ammo = "Bleeding Rounds".into();
        let attacks = c.weapon_attacks();
        let shot = attack(&attacks, "Black Iron Revolver");
        assert_eq!((shot.to_hit, shot.damage.as_str()), (10, "1d8+5"));
        assert!(shot.notes[0].starts_with("On a hit: CON save"));
    }

    #[test]
    fn toggled_modifiers() {
        let mut c = sable();
        let iron = item_index(&c, "Black Iron Revolver");
        c.inventory[iron].modifiers = vec![Modifier {
            kind: ModifierKind::Damage,
            dice: "2d6".into(),
            damage_type: "fire".into(),
            text: "Fire mode".into(),
            toggleable: true,
            enabled: false,
            ..Default::default()
        }];
        let pointer = format!("/inventory/{iron}/modifiers/0");
        let shot = attack(&c.weapon_attacks(), "Black Iron Revolver").clone();
        assert!(shot.riders.iter().all(|r| !r.name.contains("Fire mode")));
        assert_eq!(shot.toggles.len(), 1);
        assert_eq!(shot.toggles[0].pointer, pointer);
        assert_eq!(shot.toggles[0].label, "Fire mode");
        assert!(!shot.toggles[0].enabled);
        assert!(
            attack(&c.weapon_attacks(), "Revolver").toggles.is_empty(),
            "only the weapon carrying it"
        );
        assert_eq!(c.toggles().len(), 1);
        crate::edit::set(
            &mut c,
            &format!("{pointer}/enabled"),
            serde_json::json!(true),
        )
        .unwrap();
        let katana = attack(&c.weapon_attacks(), "Katana").clone();
        assert!(
            katana.riders.iter().all(|r| !r.name.contains("Fire mode")),
            "a weapon's damage stays on that weapon"
        );

        crate::edit::set(
            &mut c,
            &format!("{pointer}/enabled"),
            serde_json::json!(true),
        )
        .unwrap();
        let shot = attack(&c.weapon_attacks(), "Black Iron Revolver").clone();
        assert!(
            shot.riders
                .iter()
                .any(|r| r.name == "Black Iron Revolver (Fire mode)")
        );
        assert!(shot.toggles[0].enabled);

        // A switched-off modifier elsewhere has no effect either.
        let ac = c.armor_class();
        c.features[0].modifiers.push(Modifier {
            kind: ModifierKind::ArmorClass,
            value: 2,
            toggleable: true,
            enabled: false,
            ..Default::default()
        });
        assert_eq!(c.armor_class(), ac);
        assert_eq!(c.toggles().len(), 2);
        assert!(
            attack(&c.weapon_attacks(), "Katana").toggles.is_empty(),
            "AC switches stay off the attack rows"
        );
    }

    #[test]
    fn firing_and_reloading() {
        let mut c = sable();
        let iron = item_index(&c, "Black Iron Revolver");
        let rounds = item_index(&c, "+1 Revolver Rounds");
        for _ in 0..9 {
            c.fire(iron).unwrap();
        }
        assert_eq!(c.inventory[rounds].quantity, 51);
        assert!(matches!(c.fire(iron), Err(PlayError::Unavailable(_))));
        c.reload(iron).unwrap();
        assert_eq!(c.inventory[iron].weapon.as_ref().unwrap().loaded, 9);

        // Untracked ammunition only counts the cylinder.
        let revolver = item_index(&c, "Revolver");
        c.fire(revolver).unwrap();
        assert_eq!(c.inventory[revolver].weapon.as_ref().unwrap().loaded, 5);
        assert!(
            c.fire(item_index(&c, "Katana")).is_ok(),
            "melee weapons never run dry"
        );
    }

    #[test]
    fn riders_once_per_turn() {
        let mut c = sable();
        let hm = c
            .spells
            .iter()
            .position(|s| s.name == "Hunter's Mark")
            .unwrap();
        c.cast_spell(hm, 0).unwrap();
        let katana = attack(&c.weapon_attacks(), "Katana").clone();
        assert_eq!(
            c.weapon_damage(&katana, false, &[]).unwrap(),
            "1d8+4+1d6+1d6"
        );

        // Slash Draw's melee rider is spent; Hunter's Mark still applies.
        let katana = attack(&c.weapon_attacks(), "Katana").clone();
        assert_eq!(c.weapon_damage(&katana, true, &[]).unwrap(), "2d8+4+2d6");
        let revolver = attack(&c.weapon_attacks(), "Revolver").clone();
        assert_eq!(
            c.weapon_damage(&revolver, false, &["Hunter's Mark".into()])
                .unwrap(),
            "1d6+4+1d4"
        );

        c.new_turn();
        assert!(c.riders_used.is_empty());
        assert_eq!(c.round, 1);
    }

    #[test]
    fn effects_concentration_and_duration() {
        let mut c = sable();
        let spell =
            |c: &Character, name: &str| c.spells.iter().position(|s| s.name == name).unwrap();
        let hm = spell(&c, "Hunter's Mark");
        c.cast_spell(hm, 0).unwrap();
        assert_eq!(c.spells[hm].free_uses_spent, 1);
        assert_eq!(c.concentrating_on().unwrap().name, "Hunter's Mark");

        // Blur takes a 2nd-level slot and ends Hunter's Mark.
        let blur = spell(&c, "Blur");
        assert!(c.cast_spell(blur, 1).is_err());
        c.cast_spell(blur, 2).unwrap();
        assert_eq!(c.spell_slots_used[1], 1);
        assert_eq!(c.effects.len(), 1);
        assert_eq!(c.concentrating_on().unwrap().name, "Blur");

        // Blessing: resistance to everything until the start of the next turn.
        let blessing = c
            .features
            .iter()
            .position(|f| f.name.starts_with("Blessing"))
            .unwrap();
        let outcome = c.use_feature_action(blessing, 0).unwrap();
        assert_eq!(outcome.spent, "2 of 3 uses left");
        assert!(c.resistances().contains(&"all".to_owned()));
        c.new_turn();
        assert!(!c.resistances().contains(&"all".to_owned()));
        assert_eq!(c.effects.len(), 1, "Blur lasts until removed");

        c.use_feature_action(blessing, 0).unwrap();
        c.use_feature_action(blessing, 0).unwrap();
        assert!(matches!(
            c.use_feature_action(blessing, 0),
            Err(PlayError::Unavailable(_))
        ));

        c.end_combat();
        assert_eq!(c.round, 0);
        c.long_rest();
        assert!(c.effects.is_empty());
        assert_eq!(c.features[blessing].uses_spent, 0);
        assert_eq!(c.spells[hm].free_uses_spent, 0);
    }

    #[test]
    fn aid_raises_max_hp() {
        let mut c = sable();
        let aid = c.spells.iter().position(|s| s.name == "Aid").unwrap();
        let outcome = c.cast_spell(aid, 2).unwrap();
        c.apply_roll(outcome.roll_effect, 5);
        assert_eq!(c.max_hp(), 57);
        assert_eq!(c.hp.current, 57);
        let i = c.effects.iter().position(|e| e.name == "Aid").unwrap();
        c.end_effect(i).unwrap();
        assert_eq!(c.hp.current, 52);
    }

    #[test]
    fn item_actions_spend_their_cost() {
        let mut c = sable();
        c.hp.current = 10;
        let potion = item_index(&c, "Potion of Healing");
        let outcome = c.use_item_action(potion, 0).unwrap();
        assert_eq!(outcome.roll, "2d4+2");
        assert_eq!(outcome.roll_effect, RollEffect::Heal);
        c.apply_roll(outcome.roll_effect, 7);
        assert_eq!(c.hp.current, 17);
        c.use_item_action(potion, 0).unwrap();
        assert_eq!(c.inventory[potion].quantity, 0);
        assert!(c.use_item_action(potion, 0).is_err());

        let kit = item_index(&c, "Healer's Kit");
        c.use_item_action(kit, 0).unwrap();
        assert_eq!(c.inventory[kit].charges_spent, 1);
    }

    #[test]
    fn armor_items_and_shields() {
        let mut c = fighter_wizard();
        c.inventory.push(Item {
            name: "Chain Mail".into(),
            kind: ItemKind::Armor,
            armor: Some(ArmorKind::ChainMail),
            equipped: true,
            ..Default::default()
        });
        c.inventory.push(Item {
            name: "+1 Shield".into(),
            kind: ItemKind::Shield,
            ac_bonus: 1,
            equipped: true,
            ..Default::default()
        });
        assert_eq!(c.armor_class(), 19);
        c.apply_effect(Effect {
            name: "Shield of Faith".into(),
            concentration: true,
            modifiers: vec![Modifier {
                kind: ModifierKind::ArmorClass,
                value: 2,
                ..Default::default()
            }],
            ..Default::default()
        });
        assert_eq!(c.armor_class(), 21);
        // Unarmored 12, the shield 3 and Shield of Faith 2.
        c.inventory[0].equipped = false;
        assert_eq!(c.armor_class(), 17);
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

    fn scaling() -> Character {
        let mut c = Character::new("Scaling");
        c.classes[0].class = ClassId::Wizard;
        c.classes[0].level = 1;
        c.base_scores[Ability::Int.index()] = 16;
        c.features.push(Feature {
            name: "Stonecunning".into(),
            uses_formula: "{pb}".into(),
            actions: vec![Action {
                roll_formula: "{pb}".into(),
                ..Default::default()
            }],
            modifiers: vec![
                Modifier {
                    kind: ModifierKind::MaxHp,
                    value_formula: "{level}".into(),
                    ..Default::default()
                },
                Modifier {
                    kind: ModifierKind::Initiative,
                    value_formula: "{pb}".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        c.spells.push(Spell {
            name: "Fire Bolt".into(),
            roll_formula: "{cantrip}d10".into(),
            ..Default::default()
        });
        c.spells.push(Spell {
            name: "Cure Wounds".into(),
            level: 1,
            roll_formula: "2d8+{spell}".into(),
            ..Default::default()
        });
        c.normalize();
        c
    }

    #[test]
    fn formulas_follow_the_character() {
        let mut c = scaling();
        assert_eq!(c.features[0].uses_max, 2);
        assert_eq!(c.features[0].actions[0].roll, "2");
        assert_eq!(c.features[0].modifiers[0].value, 1);
        assert_eq!(c.features[0].modifiers[1].value, 2);
        assert_eq!(c.spells[0].roll, "1d10");
        assert_eq!(c.spells[1].roll, "2d8+3");

        crate::edit::set(&mut c, "/classes/0/level", serde_json::json!(11)).unwrap();
        assert_eq!(c.features[0].uses_max, 4);
        assert_eq!(c.features[0].modifiers[0].value, 11);
        assert_eq!(c.spells[0].roll, "3d10");
        crate::edit::set(&mut c, "/base_scores/3", serde_json::json!(8)).unwrap();
        assert_eq!(c.spells[1].roll, "2d8-1");
    }

    #[test]
    fn scaled_max_hp_counts_before_current_hp_is_clamped() {
        let mut c = scaling();
        let base = c.max_hp() - 1;
        c.hp.max = base;
        crate::edit::set(&mut c, "/classes/0/level", serde_json::json!(5)).unwrap();
        assert_eq!(c.max_hp(), base + 5);
        crate::edit::set(&mut c, "/hp/current", serde_json::json!(base + 5)).unwrap();
        assert_eq!(c.hp.current, base + 5);
    }

    #[test]
    fn clearing_a_formula_keeps_the_value_as_a_manual_one() {
        let mut c = scaling();
        crate::edit::set(&mut c, "/features/0/uses_formula", serde_json::json!("")).unwrap();
        crate::edit::set(&mut c, "/classes/0/level", serde_json::json!(11)).unwrap();
        assert_eq!(c.features[0].uses_max, 2);
    }

    #[test]
    fn a_bad_formula_is_rejected_and_changes_nothing() {
        let mut c = scaling();
        let before = c.clone();
        let bad = crate::edit::set(
            &mut c,
            "/features/0/uses_formula",
            serde_json::json!("{nope}"),
        );
        assert!(bad.is_err());
        let dice = crate::edit::set(&mut c, "/features/0/uses_formula", serde_json::json!("1d4"));
        assert!(dice.is_err());
        assert_eq!(c, before);
    }

    #[test]
    fn a_bad_formula_in_stored_data_keeps_the_last_value() {
        let mut c = scaling();
        c.features[0].uses_formula = "{nope}".into();
        c.normalize();
        assert_eq!(c.features[0].uses_max, 2);
    }

    #[test]
    fn main_spell_ability_is_the_highest_level_casting_class() {
        let mut c = Character::new("Multi");
        c.classes[0].class = ClassId::Cleric;
        c.classes[0].level = 2;
        c.classes.push(ClassLevel {
            class: ClassId::Wizard,
            level: 5,
            ..Default::default()
        });
        assert_eq!(c.main_spell_ability(), Some(Ability::Int));
        c.classes[0].class = ClassId::Barbarian;
        c.classes[1].class = ClassId::Fighter;
        assert_eq!(c.main_spell_ability(), None);
    }

    fn wand(spent: u32, regain: &str) -> Item {
        Item {
            name: "Wand".into(),
            charges_max: 7,
            charges_spent: spent,
            recharge: Recharge::Dawn,
            regain: regain.into(),
            ..Default::default()
        }
    }

    #[test]
    fn dawn_regains_rolled_charges_up_to_the_maximum() {
        let mut rng = rand::rng();
        for _ in 0..50 {
            let mut c = Character::new("A");
            c.inventory.push(wand(7, "1d6+1"));
            let rolled = c.dawn(&mut rng);
            assert_eq!(rolled.len(), 1);
            let (name, roll, left) = &rolled[0];
            assert_eq!(name, "Wand");
            assert!((2..=7).contains(&roll.total));
            assert_eq!(*left as i32, roll.total);
            assert_eq!(c.inventory[0].charges_spent, 7 - roll.total as u32);

            c.inventory[0].charges_spent = 1;
            c.dawn(&mut rng);
            assert_eq!(c.inventory[0].charges_spent, 0);
        }
    }

    #[test]
    fn dawn_skips_full_items_and_refills_without_dice() {
        let mut c = Character::new("A");
        c.inventory.push(wand(0, "1d6"));
        c.inventory.push(wand(3, ""));
        assert!(c.dawn(&mut rand::rng()).is_empty());
        assert_eq!(c.inventory[1].charges_spent, 0);
    }

    #[test]
    fn long_rest_leaves_dawn_charges_to_the_dawn() {
        let mut c = Character::new("A");
        c.inventory.push(wand(3, "1d6"));
        c.long_rest();
        assert_eq!(c.inventory[0].charges_spent, 3);
    }

    #[test]
    fn bad_regain_dice_are_rejected() {
        let mut c = Character::new("A");
        c.inventory.push(wand(3, "1d6"));
        let bad = crate::edit::set(&mut c, "/inventory/0/regain", serde_json::json!("lots"));
        assert!(bad.is_err());
        assert_eq!(c.inventory[0].regain, "1d6");
    }

    #[test]
    fn versatile_weapons_get_a_two_handed_attack() {
        let mut c = Character::new("A");
        c.base_scores[Ability::Str.index()] = 16;
        c.inventory.push(Item {
            name: "Spear".into(),
            kind: ItemKind::Weapon,
            equipped: true,
            weapon: Some(WeaponStats {
                damage: "1d6".into(),
                versatile: "1d8".into(),
                thrown: true,
                range: "20/60".into(),
                ..Default::default()
            }),
            ..Default::default()
        });
        let attacks = c.weapon_attacks();
        let names: Vec<&str> = attacks.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Spear", "Spear (thrown)", "Spear (two-handed)"]);
        assert_eq!(attacks[0].damage, "1d6+3");
        assert_eq!(attacks[2].damage, "1d8+3");
        assert_eq!(attacks[2].to_hit, attacks[0].to_hit);
        assert_eq!(attacks[2].range, "melee");
        assert_eq!(attacks[0].properties, ["Thrown (20/60)", "Versatile (1d8)"]);
    }
}
