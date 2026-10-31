//! Folding a `comptime` of literals while the program is built
//! ([ADR-287](../../../docs/specification/adr/adr-287.md) D6,
//! [ADR-321](../../../docs/specification/adr/adr-321.md) D1).
//!
//! An initialiser of literals alone - numbers, text, a list, a struct, a
//! variant, `f"…"`, an operator, an `if` whose branches are values, an index,
//! another `comptime`, a file the build reads, a grammar run on a text - is
//! folded here, by `tools/build_eval.nika`. **Nothing here calls a function**:
//! build-time code that calls one is compiled to Rust and run with the
//! program (`crate::comptime_run`), so the build-time answer is the run-time
//! one, and a check with no place to compile it refuses the value (D19).

use nikaia_std::tools::build_eval::{BtHost, BtRan, BtRead, bt_evaluate};

use crate::assets::Reads;
use crate::ast::{Expr, Item};
use crate::contracts::ty::TyOps;
use crate::parser::Parsed;

/// **What a build-time expression came to**, in Nikaia
/// (`tools/build_values.nika`, ADR-294, #125): an integer as a magnitude and
/// a sign, a float as the machine carries one, a truth value, decoded text, a
/// fixed-length list, a pair, a variant or a struct of the program's own -
/// each a kind a `const` can carry (ADR-287 D6).
pub use nikaia_std::tools::build_values::BuildValue as Value;

/// Why a build-time expression did not come to a value: declared in Nikaia
/// (`tools/build_eval.nika`, ADR-294, #125), with a grammar's wall in the
/// checker's own type.
pub use nikaia_std::tools::build_eval::BtRefusal as Refusal;

/// What a name outside a build-time body is worth: a `comptime` already
/// evaluated, or a `let` whose value folded.
/// **An integer while the program is built**: a magnitude and a sign, the
/// shape the tree gives a literal (ADR-294 D11), with what the evaluator does
/// with two of them written in Nikaia (`tools/integers.nika`).
pub use nikaia_std::tools::integers::Integer;

pub type Known<'a> = &'a dyn Fn(&str) -> Option<Value>;

/// The evaluator, over one unit's items. The evaluator itself is
/// `tools/build_eval.nika` (ADR-294, #125); this holds what it asks for - the
/// program's files, the ledger, what a build may read, and the grammar workshop.
pub struct BuildTime<'a> {
    parsed: &'a Parsed,
    /// **Every file of this program**: a variant or a grammar may be declared
    /// in any of them.
    beside: &'a [&'a Parsed],
    /// **What this build may read while it builds**
    /// ([ADR-310](../../../docs/specification/adr/adr-310.md)). A caller that
    /// passes [`Reads::none`] gets D1: the whole class is off, which is what
    /// every test and every build that did not ask for it gets.
    reads: &'a Reads,
    known: Known<'a>,
}

impl<'a> BuildTime<'a> {
    pub fn new(
        parsed: &'a Parsed,
        beside: &'a [&'a Parsed],
        reads: &'a Reads,
        known: Known<'a>,
    ) -> Self {
        Self {
            parsed,
            beside,
            reads,
            known,
        }
    }

    /// What an initialiser comes to.
    pub fn evaluate(&mut self, expr: &Expr) -> Result<Value, Refusal> {
        self.run(expr)
    }

    fn run(&self, expr: &Expr) -> Result<Value, Refusal> {
        let mut units: Vec<&Parsed> = vec![self.parsed];
        units.extend(self.beside.iter().copied());
        let host = Host {
            units,
            reads: self.reads,
            known: self.known,
        };
        bt_evaluate(expr, &host).map_err(|thrown| thrown.split().0)
    }
}

/// What the evaluator asks the compiler: the files by number (0 is the one
/// being checked), the ledger, what may be read, and the grammar workshop.
struct Host<'h> {
    units: Vec<&'h Parsed>,
    reads: &'h Reads,
    known: Known<'h>,
}

impl Host<'_> {
    /// The files in the order a name is looked for: the one the body being read
    /// came from, and then every file beside the one being checked.
    fn in_reach(&self, unit: i64) -> impl Iterator<Item = (usize, &Parsed)> {
        std::iter::once(unit as usize)
            .chain(1..self.units.len())
            .map(|at| (at, self.units[at]))
    }
}

impl BtHost for Host<'_> {
    fn text(&self, unit: i64, symbol: winnow_grammar::Symbol) -> String {
        self.units[unit as usize].text(symbol).to_string()
    }

    fn char_of(&self, n: i64) -> Option<char> {
        char_of(n)
    }

    fn known(&self, name: &str) -> Option<Value> {
        (self.known)(name)
    }

    fn constant(&self, unit: i64, name: &str) -> Option<Expr> {
        let parsed = self.units[unit as usize];
        parsed
            .program
            .items
            .iter()
            .find_map(|item| match &item.node {
                Item::Comptime {
                    name: declared,
                    value,
                    ..
                } if parsed.text(*declared) == name => Some(value.clone()),
                _ => None,
            })
    }

    fn variant_of(&self, unit: i64, ty: &str, variant: &str) -> bool {
        self.in_reach(unit).any(|(_, parsed)| {
            parsed.program.items.iter().any(|item| match &item.node {
                Item::Enum { name, variants, .. } if parsed.text(*name) == ty => variants
                    .iter()
                    .any(|held| parsed.text(held.name) == variant),
                _ => false,
            })
        })
    }

    fn run_grammar(&self, unit: i64, which: &str, rule: &str, input: &str) -> BtRan {
        let Some((parsed, result)) = self.rule_of(unit, which, rule) else {
            return BtRan {
                found: false,
                value: None,
                wall: None,
            };
        };
        let beside: Vec<&Parsed> = self.units[1..].to_vec();
        let ask = crate::grammar_run::Ask {
            grammar: which,
            rule,
            input,
            result: &result,
            units: &beside,
        };
        let outcome = self
            .reads
            .workshop()
            .run(parsed, &ask)
            .and_then(|dump| crate::grammar_run::decode(&dump));
        match outcome {
            Ok(value) => BtRan {
                found: true,
                value: Some(value),
                wall: None,
            },
            Err(why) => BtRan {
                found: true,
                value: None,
                wall: Some(why),
            },
        }
    }

    fn read(&self, path: &str) -> BtRead {
        match self.reads.read(path) {
            Ok(text) => BtRead {
                text: Some(text),
                denied: None,
            },
            Err(why) => BtRead {
                text: None,
                denied: Some(why),
            },
        }
    }
}

impl Host<'_> {
    /// The file a grammar was declared in, and what its `entry` rule hands back.
    fn rule_of(
        &self,
        unit: i64,
        grammar: &str,
        rule: &str,
    ) -> Option<(&Parsed, crate::contracts::ty::Ty)> {
        self.in_reach(unit).find_map(|(_, parsed)| {
            let found = parsed
                .program
                .items
                .iter()
                .find_map(|item| match &item.node {
                    Item::Grammar(def) if parsed.text(def.name) == grammar => def
                        .rules
                        .iter()
                        .find(|r| r.is_entry && parsed.text(r.name) == rule),
                    _ => None,
                })?;
            let ty = found.ret_type.as_ref()?;
            Some((parsed, crate::contracts::ty::Ty::from_ast(parsed, ty)))
        })
    }
}

/// An escape the set does not name, as it is written, and why.
///
/// **The one table, with two readers**
/// ([ADR-188](../../docs/specification/adr/adr-188.md) D2): [`decoded`] turns a
/// literal into a value at build time, and `Checker::an_escape_nothing_names`
/// refuses a literal the set does not cover. Both read
/// `crates/nikaia-std/src/tools/escapes.nika`, where the table is Nikaia
/// ([ADR-294](../../docs/specification/adr/adr-294.md), #125).
pub use nikaia_std::tools::escapes::NotAnEscape as Refused;

/// The escapes this language has, as a reader of a diagnostic wants them
/// ([Part I 2.5](../../docs/specification/10-nikaia-light.md)).
///
/// A list and not a sentence, because the message prints it and the page prints
/// it, and a set written twice is a set that disagrees with itself.
pub const ESCAPES: &str = "\\n \\r \\t \\0 \\\\ \\' \\\" \\xNN \\u{…}";

/// Which numbers name a character: Unicode's table, handed to the Nikaia walk.
pub(crate) fn char_of(n: i64) -> Option<char> {
    u32::try_from(n).ok().and_then(char::from_u32)
}

/// The first escape in a literal that this language's set does not name.
///
/// **Reads the same table [`decoded`] does**, which is what makes a refusal
/// here safe: everything this returns `Some` for is something `decoded`
/// returns `None` for, and `rustc` refuses in its own words.
pub fn an_escape_nothing_names(literal: &str) -> Option<Refused> {
    nikaia_std::tools::escapes::an_escape_nothing_names(literal, &char_of)
}

/// **What a written string literal means** — the value behind the spelling.
///
/// The parser keeps a literal's escapes (`STR_CHAR` takes `\` and any
/// character) and the emitter hands the text to `rustc` unchanged, so **this
/// language's escapes are Rust's**, decided by what that compiler accepts
/// rather than by a page here. So this is a **faithful reading** and not a
/// second definition. `None` where the escape is one `rustc` would reject:
/// that program does not compile either way, and the evaluator says *cannot
/// evaluate* rather than inventing a meaning for it.
///
/// [`written`] is the inverse, and `crates/nikaia/tests/build_time.rs` holds
/// the pair to the only standard that settles it: the same literal, read at
/// build time and at run time, printing the same bytes.
pub fn decoded(literal: &str) -> Option<String> {
    nikaia_std::tools::escapes::decoded(literal, &char_of)
}

/// The inverse of [`decoded`]: a value, spelled as a literal `rustc` reads.
/// Every control character is written as an escape.
pub fn written(text: &str) -> String {
    nikaia_std::tools::escapes::as_a_literal(text)
}

/// **Why a function may not run while the program is built**, read off its
/// ledger entry ([ADR-287](../../../docs/specification/adr/adr-287.md) D13,
/// D14), by `tools/build_eval.nika`. Nothing where it may run.
pub(crate) fn may_not_run(contract: &crate::contracts::FnContract) -> Option<String> {
    nikaia_std::tools::build_eval::bt_may_not_run(contract)
}
