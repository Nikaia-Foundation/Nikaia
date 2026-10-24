// crates/nikaia/src/fold_bound.rs
//
// **A fold over a grammar's input takes no more steps than the input has room
// for; its counters are proved from that**
// ([ADR-367](../../docs/specification/adr/adr-367.md)).
//
// Three parts, each a decision of the record:
//
// * [`least_width`] (D1): the fewest bytes any match of a rule takes, read
//   from the grammar. A fold's step consumes at least `max(w, 1)` of them.
// * [`Extent`] (D2): whether the input's length is a fact when the fold
//   starts. It is for every input a grammar reads today, and is not for one
//   that arrives while it parses.
// * [`proven_counters`] (D3): the whole-number fields of the accumulator that
//   every step changes by at most `k`, outside any loop, and that the merge
//   leaves the sum of both partial results' fields or one of them. The
//   invariant `|f| <= k * steps` is checked by the solver before it is used
//   (ADR-361 D4), and a field's checks are proved where `k * 2^60 / max(w, 1)`
//   fits its type.


use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, FoldSpec, GrammarDef, Item, Pattern};
use crate::parser::Parsed;
use nikaia_std::tools::fold_bound::{
    FbHost, FbSource, fb_fold_counters, fb_folds, fb_index, fb_least_width, fb_most_steps,
    fb_uses, fb_whole_changes_itself,
};
use nikaia_std::tools::prover_arena::TermArena;

/// What the Nikaia walk asks of the compiler: the holes of a literal, and the
/// solver.
struct Host<'a> {
    parsed: &'a Parsed,
}

impl FbHost for Host<'_> {
    fn literal_holes(&self, literal: &Expr) -> Vec<Expr> {
        crate::emit::literal_expressions(self.parsed, literal)
    }

    fn ask(&self, arena: &TermArena, facts: &[i64], goal: i64) -> bool {
        crate::proofs::ask(arena, facts, goal) == crate::proofs::Asked::Proved
    }
}

fn source(parsed: &Parsed) -> FbSource<'_, '_> {
    FbSource {
        program: &parsed.program,
        words: &parsed.interner,
    }
}

// ---------------------------------------------------------------------------
// D1: the least width of a rule
// ---------------------------------------------------------------------------

/// The fewest bytes any match of a rule of a grammar takes. `None` where the
/// grammar or the rule is not in the program.
pub fn least_width(parsed: &Parsed, grammar: &str, rule: &str) -> Option<u64> {
    fb_least_width(&source(parsed), grammar, rule)
}

// ---------------------------------------------------------------------------
// D2: the input's extent
// ---------------------------------------------------------------------------

/// The least `len` below which every input of known extent stays: the length
/// of every text and every `Bytes` is below 2^60 (Part III, `optimization`).
pub const LENGTH_BOUND: u128 = 1 << 60;

/// **Whether a fold's input has a length when the fold starts**
/// ([ADR-367](../../docs/specification/adr/adr-367.md) D2).
///
/// A fold's step count, and every proof of D3 built on it, starts from this.
///
/// **WARNING, for whoever adds a kind of input.** Every input a grammar reads
/// today is held whole before the parse starts - text (a `String` or a
/// mapping) or a `Bytes` - and is [`Extent::Known`]. A grammar over input
/// that **arrives while it parses** (winnow's `Partial`, a stream, a socket)
/// must give [`Extent::Unbounded`]: its length is not known when the fold
/// starts and has no bound, so no step count exists and no counter may be
/// proved from one. **A new kind of input starts as `Unbounded`** until a
/// record decides otherwise; making it `Known` without one would remove
/// overflow checks a stream needs, and an overflow would then run on silently
/// into a wrong result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extent {
    /// Held whole before the parse: its length is a value below
    /// [`LENGTH_BOUND`].
    Known,
    /// Arrives while it is parsed: no length, no bound, no step count.
    Unbounded,
}

/// What a grammar's entry reads. Today every entry takes text or `Bytes`
/// (ADR-366 D1), both held whole.
///
/// **ADR-367 D2: this is the one place that decides the extent.** An entry
/// that reads input arriving while it parses (a winnow `Partial`, a stream)
/// must answer [`Extent::Unbounded`] here, and so must any kind of input this
/// function does not know yet: there is no step count over a stream.
pub fn input_extent(_grammar: &GrammarDef) -> Extent {
    // No grammar reads a stream today (ADR-366 D1): text and `Bytes` only.
    // When one does, its entry is told apart here and gives `Unbounded`.
    Extent::Known
}

/// **The most steps a fold over an input of this extent takes** (D1, D2):
/// `len / max(w, 1)` with `len` below [`LENGTH_BOUND`], and nothing - no
/// proof - where the extent is unbounded.
pub fn most_steps(extent: Extent, width: u64) -> Option<u128> {
    match extent {
        Extent::Known => Some(u128::from(fb_most_steps(width))),
        // ADR-367 D2: a stream has no length, so no step count.
        Extent::Unbounded => None,
    }
}

// ---------------------------------------------------------------------------
// D3: growth per step
// ---------------------------------------------------------------------------

/// The folds a pattern holds.
pub(crate) fn folds<'p>(pattern: &'p Pattern, out: &mut Vec<&'p FoldSpec>) {
    out.extend(fb_folds(pattern));
}

/// **The counters of every fold over a grammar's input that ADR-367 D3
/// proves**, as the compound assignments whose check is not needed, by the
/// statement's first byte. `arithmetic` is the checker's table of the `+`,
/// `-` and `*` whose two sides are one whole-number type, with that type.
pub fn proven_counters(parsed: &Parsed, arithmetic: &BTreeMap<i64, String>) -> BTreeSet<i64> {
    let source = source(parsed);
    let mut out = BTreeSet::new();
    // A method of a whole number that changes it in place changes a field
    // where no write shows it.
    if fb_whole_changes_itself(&source) {
        return out;
    }
    let host = Host { parsed };
    let ix = fb_index(&source);
    let uses = fb_uses(&host, &ix, &source);
    for item in &parsed.program.items {
        let Item::Grammar(grammar) = &item.node else {
            continue;
        };
        // ADR-367 D2: the step count starts from the input's extent. A
        // grammar over input that arrives while it parses gives
        // `Extent::Unbounded`, and nothing below is proved for it.
        let known = input_extent(grammar) == Extent::Known;
        for rule in &grammar.rules {
            let mut specs = Vec::new();
            for alt in &rule.alts {
                folds(&alt.pattern.node, &mut specs);
            }
            for spec in specs {
                let at = rule.span.start as i64;
                out.extend(crate::proofs::in_function(
                    parsed.text(rule.name),
                    at as usize,
                    || {
                        fb_fold_counters(
                            &host, &ix, &uses, &source, grammar, at, spec, known, arithmetic,
                        )
                    },
                ));
            }
        }
    }
    out
}
