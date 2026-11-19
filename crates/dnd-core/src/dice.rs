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
    /// Where this term came from, e.g. `DEX` or `Hunter's Mark`; empty for free-form dice
    /// and rolls logged before sources were tracked.
    #[serde(default)]
    pub source: String,
    /// The type of damage this term deals, if any.
    #[serde(default)]
    pub damage_type: String,
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

/// One labelled piece of a roll, such as a weapon's dice or a bonus from a feature. Every term
/// of `expression` carries the part's source and damage type through to the result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RollPart {
    pub expression: String,
    pub source: String,
    pub damage_type: String,
}

impl RollPart {
    pub fn new(expression: impl Into<String>, source: impl Into<String>) -> Self {
        RollPart {
            expression: expression.into(),
            source: source.into(),
            damage_type: String::new(),
        }
    }

    pub fn typed(mut self, damage_type: impl Into<String>) -> Self {
        self.damage_type = damage_type.into();
        self
    }

    /// The part with every die count doubled, for critical hits.
    pub fn doubled(mut self) -> Result<Self, DiceError> {
        if !self.expression.trim().is_empty() {
            self.expression = double_dice(&self.expression)?;
        }
        Ok(self)
    }
}

/// A flat number added to a roll and what it comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bonus {
    pub value: i32,
    pub source: String,
}

impl Bonus {
    pub fn new(value: i32, source: impl Into<String>) -> Self {
        Bonus {
            value,
            source: source.into(),
        }
    }

    pub fn part(&self) -> RollPart {
        RollPart::new(self.value.to_string(), self.source.clone())
    }

    pub fn total(bonuses: &[Bonus]) -> i32 {
        bonuses.iter().map(|b| b.value).sum()
    }
}

impl RollResult {
    /// The total dealt of each damage type, in the order they first appear; empty unless
    /// some term has a type. Terms without one are grouped under an empty name, so the
    /// amounts always add up to the total.
    pub fn damage_by_type(&self) -> Vec<(String, i32)> {
        if self.terms.iter().all(|t| t.damage_type.is_empty()) {
            return Vec::new();
        }
        let mut out: Vec<(String, i32)> = Vec::new();
        for term in &self.terms {
            let amount = term.sign * term.value;
            match out.iter_mut().find(|(t, _)| *t == term.damage_type) {
                Some((_, sum)) => *sum += amount,
                None => out.push((term.damage_type.clone(), amount)),
            }
        }
        out
    }

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
                    source: String::new(),
                    damage_type: String::new(),
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
    let (terms, total, natural) = roll_expr(Expr::parse(expression)?.with_mode(mode), rng);
    Ok(RollResult {
        expression: expression.trim().to_owned(),
        mode,
        terms,
        total,
        natural,
    })
}

/// Rolls labelled parts together with the thread-local RNG.
pub fn roll_parts(parts: &[RollPart], mode: RollMode) -> Result<RollResult, DiceError> {
    roll_parts_with(parts, mode, &mut rand::rng())
}

/// Rolls labelled parts as one roll, each term keeping its part's source and damage type.
/// Parts that add nothing are left out of the result.
pub fn roll_parts_with(
    parts: &[RollPart],
    mode: RollMode,
    rng: &mut impl Rng,
) -> Result<RollResult, DiceError> {
    let (terms, owners) = parsed_parts(parts)?;
    let expression = compact(&terms);
    let (mut results, total, natural) = roll_expr(Expr { terms }.with_mode(mode), rng);
    for (result, part) in results.iter_mut().zip(owners) {
        result.source = part.source.clone();
        result.damage_type = part.damage_type.clone();
    }
    Ok(RollResult {
        expression,
        mode,
        terms: results,
        total,
        natural,
    })
}

/// The compact expression for `parts`, as `RollResult::expression` will show it.
pub fn expression_of(parts: &[RollPart]) -> Result<String, DiceError> {
    Ok(compact(&parsed_parts(parts)?.0))
}

type ParsedParts<'a> = (Vec<(i32, Term)>, Vec<&'a RollPart>);

/// Every term of `parts` that adds something, with the part it came from.
fn parsed_parts(parts: &[RollPart]) -> Result<ParsedParts<'_>, DiceError> {
    let mut terms = Vec::new();
    let mut owners = Vec::new();
    for part in parts.iter().filter(|p| !p.expression.trim().is_empty()) {
        for (sign, term) in Expr::parse(&part.expression)?.terms {
            if term != Term::Constant(0) {
                terms.push((sign, term));
                owners.push(part);
            }
        }
    }
    if terms.is_empty() {
        // Nothing but zeroes is a roll of 0, and no parts at all isn't a roll.
        let first = parts.iter().find(|p| !p.expression.trim().is_empty());
        let owner = first.ok_or(DiceError::Empty)?;
        return Ok((vec![(1, Term::Constant(0))], vec![owner]));
    }
    Ok((terms, owners))
}

/// The dice in order, then the sum of the constants: `1d8+1d6+5`.
fn compact(terms: &[(i32, Term)]) -> String {
    let mut out = String::new();
    let mut flat = 0;
    for (sign, term) in terms {
        match term {
            Term::Constant(n) => flat += sign * n,
            dice => {
                if *sign < 0 {
                    out.push('-');
                } else if !out.is_empty() {
                    out.push('+');
                }
                out.push_str(&dice.to_string());
            }
        }
    }
    match flat {
        0 if out.is_empty() => "0".to_owned(),
        0 => out,
        n if n > 0 && !out.is_empty() => format!("{out}+{n}"),
        n => format!("{out}{n}"),
    }
}

fn is_d20_test(term: &Term) -> bool {
    match *term {
        Term::Dice {
            count: 1,
            sides: 20,
            ..
        } => true,
        Term::Dice {
            count: 2,
            sides: 20,
            keep,
        } => keep != Keep::All,
        _ => false,
    }
}

fn roll_expr(expr: Expr, rng: &mut impl Rng) -> (Vec<TermResult>, i32, Option<u32>) {
    let (terms, total) = expr.roll(rng);
    let natural = expr
        .terms
        .iter()
        .zip(&terms)
        .find(|((_, term), _)| is_d20_test(term))
        .and_then(|(_, t)| t.rolls.iter().find(|(_, kept)| *kept).map(|(r, _)| *r));
    (terms, total, natural)
}

/// The expression with every die count doubled, for critical hits: `1d8+1d6+4` → `2d8+2d6+4`.
pub fn double_dice(expression: &str) -> Result<String, DiceError> {
    let expr = Expr::parse(expression)?;
    let mut out = String::new();
    for (i, (sign, term)) in expr.terms.iter().enumerate() {
        match (i, *sign) {
            (0, -1) => out.push('-'),
            (0, _) => {}
            (_, -1) => out.push('-'),
            _ => out.push('+'),
        }
        let doubled = match *term {
            Term::Dice { count, sides, keep } => Term::Dice {
                count: count * 2,
                sides,
                keep,
            },
            Term::Constant(n) => Term::Constant(n),
        };
        out.push_str(&doubled.to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn doubles_dice_for_crits() {
        assert_eq!(double_dice("1d8+1d6+4").unwrap(), "2d8+2d6+4");
        assert_eq!(double_dice("2d6 - 1").unwrap(), "4d6-1");
        assert!(double_dice("nope").is_err());
    }

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

    fn seeded() -> rand::rngs::StdRng {
        use rand::SeedableRng;
        rand::rngs::StdRng::seed_from_u64(7)
    }

    #[test]
    fn parts_tag_their_terms() {
        let parts = [
            RollPart::new("1d8", "Longsword").typed("slashing"),
            RollPart::new("3", "STR").typed("slashing"),
            RollPart::new("1d6+1", "Flame Tongue").typed("fire"),
            RollPart::new("-1", "Curse").typed("fire"),
            RollPart::new("0", "Magic bonus").typed("slashing"),
        ];
        let r = roll_parts_with(&parts, RollMode::Normal, &mut seeded()).unwrap();
        assert_eq!(r.expression, "1d8+1d6+3");
        let sources: Vec<_> = r.terms.iter().map(|t| t.source.as_str()).collect();
        assert_eq!(
            sources,
            ["Longsword", "STR", "Flame Tongue", "Flame Tongue", "Curse"]
        );
        assert_eq!(
            r.total,
            r.terms.iter().map(|t| t.sign * t.value).sum::<i32>()
        );
        let by_type = r.damage_by_type();
        assert_eq!(by_type.len(), 2);
        assert_eq!(by_type[0], ("slashing".to_owned(), r.terms[0].value + 3));
        assert_eq!(by_type[1], ("fire".to_owned(), r.terms[2].value));
    }

    #[test]
    fn parts_take_advantage_and_keep_the_natural() {
        let parts = [RollPart::new("1d20", "d20"), RollPart::new("5", "DEX")];
        let r = roll_parts_with(&parts, RollMode::Advantage, &mut seeded()).unwrap();
        assert_eq!(r.expression, "1d20+5");
        assert_eq!(r.terms[0].label, "2d20kh1");
        assert_eq!(r.terms[0].source, "d20");
        assert_eq!(r.natural, Some(r.terms[0].value as u32));
        assert!(r.damage_by_type().is_empty());
    }

    #[test]
    fn parts_edge_cases() {
        let none = roll_parts_with(&[], RollMode::Normal, &mut seeded());
        assert_eq!(none, Err(DiceError::Empty));
        let zero = [RollPart::new("0", "STR")];
        let r = roll_parts_with(&zero, RollMode::Normal, &mut seeded()).unwrap();
        assert_eq!((r.expression.as_str(), r.total), ("0", 0));
        let minus = [RollPart::new("-1d4", "Bane"), RollPart::new("-2", "Hex")];
        assert_eq!(expression_of(&minus).unwrap(), "-1d4-2");
        // A blank weapon die shouldn't stop the rest of the damage rolling.
        let blank = [RollPart::new("", "Fist"), RollPart::new("3", "STR")];
        assert_eq!(expression_of(&blank).unwrap(), "3");
    }

    #[test]
    fn natural_ignores_other_dice() {
        let r = roll_with("1d200", RollMode::Normal, &mut seeded()).unwrap();
        assert_eq!(r.natural, None);
    }

    #[test]
    fn untyped_terms_keep_damage_by_type_adding_up() {
        let parts = [
            RollPart::new("2", "Fire").typed("fire"),
            RollPart::new("3", "Other"),
        ];
        let r = roll_parts_with(&parts, RollMode::Normal, &mut seeded()).unwrap();
        assert_eq!(
            r.damage_by_type(),
            [("fire".to_owned(), 2), (String::new(), 3)]
        );
    }

    #[test]
    fn terms_logged_before_sources_load() {
        let old = r#"[{"sign":1,"label":"1d6","rolls":[[3,true]],"value":3}]"#;
        let terms: Vec<TermResult> = serde_json::from_str(old).unwrap();
        assert_eq!(terms[0].source, "");
        assert_eq!(terms[0].damage_type, "");
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
                    source: String::new(),
                    damage_type: String::new(),
                },
                TermResult {
                    sign: 1,
                    label: "3".into(),
                    rolls: vec![],
                    value: 3,
                    source: String::new(),
                    damage_type: String::new(),
                },
            ],
            total: 20,
            natural: Some(17),
        };
        assert_eq!(r.breakdown(), "[~4~, 17] + 3");
    }
}
