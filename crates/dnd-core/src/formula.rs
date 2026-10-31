//! Formulas: values that follow the character, such as `{pb}` uses or `{cantrip}d10` damage.
//!
//! A formula is a dice expression with `{variable}` placeholders. `Character::normalize`
//! resolves each one into the plain field beside it (`uses_max`, `roll`, `value`), so the rest
//! of the rules only ever see ordinary numbers and dice.

use crate::abilities::Ability;
use crate::character::Character;
use crate::dice::{DiceError, Expr, Term};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormulaError {
    #[error("unknown variable {{{0}}} in formula")]
    UnknownVariable(String),
    #[error("unclosed {{ in formula")]
    Unclosed,
    #[error("formula {0:?} can't contain dice here")]
    HasDice(String),
    #[error("bad formula {formula:?}: {source}")]
    Dice { formula: String, source: DiceError },
}

/// The numbers a formula can refer to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vars {
    pub pb: i32,
    pub level: i32,
    /// Cantrip damage dice: 1, then one more at levels 5, 11 and 17.
    pub cantrip: i32,
    /// The spellcasting modifier of the highest-level casting class, or 0.
    pub spell: i32,
    /// Ability modifiers in `Ability::ALL` order.
    pub abilities: [i32; 6],
}

impl Vars {
    pub fn of(c: &Character) -> Vars {
        let level = c.total_level();
        Vars {
            pb: c.proficiency_bonus(),
            level: level as i32,
            cantrip: match level {
                ..=4 => 1,
                5..=10 => 2,
                11..=16 => 3,
                _ => 4,
            },
            spell: c.main_spell_ability().map_or(0, |a| c.ability_mod(a)),
            abilities: Ability::ALL.map(|a| c.ability_mod(a)),
        }
    }

    fn get(&self, name: &str) -> Option<i32> {
        Some(match name {
            "pb" => self.pb,
            "level" => self.level,
            "cantrip" => self.cantrip,
            "spell" => self.spell,
            "str" => self.abilities[Ability::Str.index()],
            "dex" => self.abilities[Ability::Dex.index()],
            "con" => self.abilities[Ability::Con.index()],
            "int" => self.abilities[Ability::Int.index()],
            "wis" => self.abilities[Ability::Wis.index()],
            "cha" => self.abilities[Ability::Cha.index()],
            _ => return None,
        })
    }
}

/// Replaces each `{variable}`. A negative value after an operator folds into it, so
/// `2d8+{spell}` with a modifier of -1 gives `2d8-1` rather than an unparseable `2d8+-1`.
fn substitute(formula: &str, vars: &Vars) -> Result<String, FormulaError> {
    let mut out = String::new();
    let mut rest = formula;
    while let Some((before, after)) = rest.split_once('{') {
        out.push_str(before);
        let (name, tail) = after.split_once('}').ok_or(FormulaError::Unclosed)?;
        let name = name.trim();
        let value = vars
            .get(name)
            .ok_or_else(|| FormulaError::UnknownVariable(name.to_owned()))?;
        if value < 0 {
            match out.trim_end().chars().last() {
                Some('+') => {
                    out.truncate(out.trim_end().len() - 1);
                    out.push_str(&value.to_string());
                }
                Some('-') => {
                    out.truncate(out.trim_end().len() - 1);
                    out.push('+');
                    out.push_str(&value.abs().to_string());
                }
                _ => out.push_str(&value.to_string()),
            }
        } else {
            out.push_str(&value.to_string());
        }
        rest = tail;
    }
    out.push_str(rest);
    Ok(out)
}

fn parsed(text: &str, formula: &str) -> Result<Expr, FormulaError> {
    Expr::parse(text).map_err(|source| FormulaError::Dice {
        formula: formula.to_owned(),
        source,
    })
}

/// Resolves a formula to a dice expression, e.g. `{cantrip}d10` to `2d10`.
pub fn dice(formula: &str, vars: &Vars) -> Result<String, FormulaError> {
    let text = substitute(formula, vars)?;
    parsed(&text, formula)?;
    Ok(text.split_whitespace().collect())
}

/// Resolves a formula that has to come out as a plain number, e.g. `{pb}` or `1+{cha}`.
pub fn number(formula: &str, vars: &Vars) -> Result<i32, FormulaError> {
    parsed(&substitute(formula, vars)?, formula)?
        .terms
        .iter()
        .try_fold(0, |total, (sign, term)| match term {
            Term::Constant(n) => Ok(total + sign * n),
            Term::Dice { .. } => Err(FormulaError::HasDice(formula.to_owned())),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Vars {
        Vars {
            pb: 3,
            level: 7,
            cantrip: 2,
            spell: -1,
            abilities: [0, 1, 2, 3, 4, 5],
        }
    }

    #[test]
    fn numbers() {
        assert_eq!(number("{pb}", &vars()), Ok(3));
        assert_eq!(number("1 + { cha }", &vars()), Ok(6));
        assert_eq!(number("{level}", &vars()), Ok(7));
        assert_eq!(number("{spell}", &vars()), Ok(-1));
    }

    #[test]
    fn dice_expressions() {
        assert_eq!(dice("{cantrip}d10", &vars()).unwrap(), "2d10");
        assert_eq!(dice("2d8 + {wis}", &vars()).unwrap(), "2d8+4");
    }

    #[test]
    fn negative_values_fold_into_the_operator() {
        assert_eq!(dice("2d8+{spell}", &vars()).unwrap(), "2d8-1");
        assert_eq!(dice("2d8 + {spell}", &vars()).unwrap(), "2d8-1");
        assert_eq!(dice("2d8-{spell}", &vars()).unwrap(), "2d8+1");
    }

    #[test]
    fn mistakes_are_errors() {
        assert_eq!(
            number("{nope}", &vars()),
            Err(FormulaError::UnknownVariable("nope".into()))
        );
        assert_eq!(number("{pb", &vars()), Err(FormulaError::Unclosed));
        assert_eq!(
            number("1d4", &vars()),
            Err(FormulaError::HasDice("1d4".into()))
        );
        assert!(dice("banana", &vars()).is_err());
    }
}
