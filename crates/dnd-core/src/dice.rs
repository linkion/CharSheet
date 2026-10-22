//! Dice expressions such as `1d20+5`, `2d6 + 1d4 - 1`, `4d6kh3` or `2d20kl1`.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;

const MAX_DICE: u32 = 100;
const MAX_SIDES: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RollMode {
    #[default]
    Normal,
    Advantage,
    Disadvantage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    All,
    Highest(u32),
    Lowest(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Dice { count: u32, sides: u32, keep: Keep },
    Constant(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    /// Each term with its sign (+1 or -1).
    pub terms: Vec<(i32, Term)>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DiceError {
    #[error("empty dice expression")]
    Empty,
    #[error("unexpected '{0}' in dice expression")]
    Unexpected(String),
    #[error("at most {MAX_DICE} dice with up to {MAX_SIDES} sides")]
    TooLarge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TermResult {
    pub sign: i32,
    pub label: String,
    /// Every die rolled, with whether it was kept.
    pub rolls: Vec<(u32, bool)>,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RollResult {
    pub expression: String,
    pub mode: RollMode,
    pub terms: Vec<TermResult>,
    pub total: i32,
    /// The kept d20 when the roll was a single d20 test, for crit and fumble display.
    pub natural: Option<u32>,
}

impl RollResult {
    pub fn breakdown(&self) -> String {
        let mut out = String::new();
        for (i, term) in self.terms.iter().enumerate() {
            match (i, term.sign) {
                (0, -1) => out.push('-'),
                (0, _) => {}
                (_, -1) => out.push_str(" - "),
                _ => out.push_str(" + "),
            }
            if term.rolls.is_empty() {
                out.push_str(&term.value.to_string());
            } else {
                let dice: Vec<String> = term
                    .rolls
                    .iter()
                    .map(|&(r, kept)| {
                        if kept {
                            r.to_string()
                        } else {
                            format!("~{r}~")
                        }
                    })
                    .collect();
                out.push_str(&format!("[{}]", dice.join(", ")));
            }
        }
        out
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Constant(n) => write!(f, "{n}"),
            Term::Dice { count, sides, keep } => {
                write!(f, "{count}d{sides}")?;
                match keep {
                    Keep::All => Ok(()),
                    Keep::Highest(n) => write!(f, "kh{n}"),
                    Keep::Lowest(n) => write!(f, "kl{n}"),
                }
            }
        }
    }
}

fn parse_number(s: &str) -> Result<u32, DiceError> {
    s.parse().map_err(|_| DiceError::Unexpected(s.to_owned()))
}

fn parse_term(raw: &str) -> Result<Term, DiceError> {
    let lower = raw.to_ascii_lowercase();
    let Some((count, rest)) = lower.split_once('d') else {
        return Ok(Term::Constant(parse_number(&lower)? as i32));
    };
    let count = if count.is_empty() {
        1
    } else {
        parse_number(count)?
    };
    let (sides, keep) = if let Some((sides, n)) = rest.split_once("kh") {
        (sides, Keep::Highest(parse_number(n)?))
    } else if let Some((sides, n)) = rest.split_once("kl") {
        (sides, Keep::Lowest(parse_number(n)?))
    } else {
        (rest, Keep::All)
    };
    let sides = parse_number(sides)?;
    if count == 0 || sides == 0 || count > MAX_DICE || sides > MAX_SIDES {
        return Err(DiceError::TooLarge);
    }
    Ok(Term::Dice { count, sides, keep })
}

impl Expr {
    pub fn parse(input: &str) -> Result<Expr, DiceError> {
        let compact: String = input.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.is_empty() {
            return Err(DiceError::Empty);
        }
        let mut terms = Vec::new();
        let mut sign = 1;
        let mut current = String::new();
        for c in compact.chars() {
            match c {
                '+' | '-' => {
                    if current.is_empty() && !terms.is_empty() {
                        return Err(DiceError::Unexpected(c.to_string()));
                    }
                    if !current.is_empty() {
                        terms.push((sign, parse_term(&current)?));
                        current.clear();
                    }
                    sign = if c == '-' { -1 } else { 1 };
                }
                c if c.is_ascii_alphanumeric() => current.push(c),
                c => return Err(DiceError::Unexpected(c.to_string())),
            }
        }
        if current.is_empty() {
            return Err(DiceError::Unexpected("end of expression".into()));
        }
        terms.push((sign, parse_term(&current)?));
        Ok(Expr { terms })
    }

    /// Applies advantage or disadvantage to the first single d20 in the expression.
    pub fn with_mode(mut self, mode: RollMode) -> Expr {
        let keep = match mode {
            RollMode::Normal => return self,
            RollMode::Advantage => Keep::Highest(1),
            RollMode::Disadvantage => Keep::Lowest(1),
        };
        if let Some((_, term)) = self.terms.iter_mut().find(|(_, t)| {
            matches!(
                t,
                Term::Dice {
                    count: 1,
                    sides: 20,
                    keep: Keep::All
                }
            )
        }) {
            *term = Term::Dice {
                count: 2,
                sides: 20,
                keep,
            };
        }
        self
    }

    pub fn roll(&self, rng: &mut impl Rng) -> (Vec<TermResult>, i32) {
        let mut total = 0;
        let results = self
            .terms
            .iter()
            .map(|(sign, term)| {
                let (rolls, value) = match *term {
                    Term::Constant(n) => (Vec::new(), n),
                    Term::Dice { count, sides, keep } => {
                        let values: Vec<u32> =
                            (0..count).map(|_| rng.random_range(1..=sides)).collect();
                        let kept = kept_flags(&values, keep);
                        let sum = values
                            .iter()
                            .zip(&kept)
                            .filter(|(_, k)| **k)
                            .map(|(v, _)| *v as i32)
                            .sum();
                        (values.into_iter().zip(kept).collect(), sum)
                    }
                };
                total += sign * value;
                TermResult {
                    sign: *sign,
                    label: term.to_string(),
                    rolls,
                    value,
                }
            })
            .collect();
        (results, total)
    }
}

fn kept_flags(values: &[u32], keep: Keep) -> Vec<bool> {
    let (n, highest) = match keep {
        Keep::All => return vec![true; values.len()],
        Keep::Highest(n) => (n as usize, true),
        Keep::Lowest(n) => (n as usize, false),
    };
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by_key(|&i| values[i]);
    if highest {
        order.reverse();
    }
    let mut kept = vec![false; values.len()];
    for &i in order.iter().take(n) {
        kept[i] = true;
    }
    kept
}

/// Parses and rolls an expression with the thread-local RNG.
pub fn roll(expression: &str, mode: RollMode) -> Result<RollResult, DiceError> {
    roll_with(expression, mode, &mut rand::rng())
}

pub fn roll_with(
    expression: &str,
    mode: RollMode,
    rng: &mut impl Rng,
) -> Result<RollResult, DiceError> {
    let expr = Expr::parse(expression)?.with_mode(mode);
    let (terms, total) = expr.roll(rng);
    let natural = terms
        .iter()
        .find(|t| t.label.starts_with("1d20") || t.label.starts_with("2d20k"))
        .and_then(|t| t.rolls.iter().find(|(_, kept)| *kept).map(|(r, _)| *r));
    Ok(RollResult {
        expression: expression.trim().to_owned(),
        mode,
        terms,
        total,
        natural,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn parses_expressions() {
        let e = Expr::parse("2d6 + 1d4 - 1").unwrap();
        assert_eq!(
            e.terms,
            vec![
                (
                    1,
                    Term::Dice {
                        count: 2,
                        sides: 6,
                        keep: Keep::All
                    }
                ),
                (
                    1,
                    Term::Dice {
                        count: 1,
                        sides: 4,
                        keep: Keep::All
                    }
                ),
                (-1, Term::Constant(1)),
            ]
        );
        assert_eq!(
            Expr::parse("d20").unwrap().terms,
            vec![(
                1,
                Term::Dice {
                    count: 1,
                    sides: 20,
                    keep: Keep::All
                }
            )]
        );
        assert_eq!(
            Expr::parse("4D6kh3").unwrap().terms[0].1,
            Term::Dice {
                count: 4,
                sides: 6,
                keep: Keep::Highest(3)
            }
        );
        assert_eq!(
            Expr::parse("-2").unwrap().terms,
            vec![(-1, Term::Constant(2))]
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(Expr::parse("  "), Err(DiceError::Empty));
        assert!(Expr::parse("1d20++2").is_err());
        assert!(Expr::parse("1d20+").is_err());
        assert!(Expr::parse("1d20*2").is_err());
        assert!(Expr::parse("abc").is_err());
        assert_eq!(Expr::parse("1000d6"), Err(DiceError::TooLarge));
        assert_eq!(Expr::parse("1d0"), Err(DiceError::TooLarge));
    }

    #[test]
    fn totals_stay_in_range() {
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..500 {
            let r = roll_with("2d6+3", RollMode::Normal, &mut rng).unwrap();
            assert!((5..=15).contains(&r.total));
            let r = roll_with("4d6kh3", RollMode::Normal, &mut rng).unwrap();
            assert!((3..=18).contains(&r.total));
            assert_eq!(r.terms[0].rolls.iter().filter(|(_, k)| *k).count(), 3);
        }
    }

    #[test]
    fn advantage_keeps_the_highest_d20() {
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..200 {
            let r = roll_with("1d20+5", RollMode::Advantage, &mut rng).unwrap();
            let rolls: Vec<u32> = r.terms[0].rolls.iter().map(|(v, _)| *v).collect();
            assert_eq!(rolls.len(), 2);
            assert_eq!(r.natural, Some(*rolls.iter().max().unwrap()));
            assert_eq!(r.total, r.natural.unwrap() as i32 + 5);

            let r = roll_with("1d20", RollMode::Disadvantage, &mut rng).unwrap();
            let rolls: Vec<u32> = r.terms[0].rolls.iter().map(|(v, _)| *v).collect();
            assert_eq!(r.natural, Some(*rolls.iter().min().unwrap()));
        }
    }

    #[test]
    fn breakdown_marks_dropped_dice() {
        let r = RollResult {
            expression: "2d20kh1+3".into(),
            mode: RollMode::Advantage,
            terms: vec![
                TermResult {
                    sign: 1,
                    label: "2d20kh1".into(),
                    rolls: vec![(4, false), (17, true)],
                    value: 17,
                },
                TermResult {
                    sign: 1,
                    label: "3".into(),
                    rolls: vec![],
                    value: 3,
                },
            ],
            total: 20,
            natural: Some(17),
        };
        assert_eq!(r.breakdown(), "[~4~, 17] + 3");
    }
}
