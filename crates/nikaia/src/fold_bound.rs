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

use crate::ast::{GrammarDef, GrammarRule, Item, Pattern, Repeat};
use crate::parser::Parsed;

// ---------------------------------------------------------------------------
// D1: the least width of a rule
// ---------------------------------------------------------------------------

/// The bytes a literal written in the source matches: an escape is one byte,
/// a `\u{…}` at least one, which only ever counts fewer than it matches.
fn literal_bytes(written: &str) -> u64 {
    let mut chars = written.chars();
    let mut bytes = 0u64;
    while let Some(c) = chars.next() {
        if c != '\\' {
            bytes += c.len_utf8() as u64;
            continue;
        }
        bytes += 1;
        if chars.next() == Some('u') {
            for d in chars.by_ref() {
                if d == '}' {
                    break;
                }
            }
        }
    }
    bytes
}

/// What D1 reads of one grammar: its rules by name, and the frame boundary a
/// `frame_end` stands for.
struct Widths<'a> {
    parsed: &'a Parsed,
    rules: BTreeMap<String, &'a GrammarRule>,
    known: BTreeMap<String, u64>,
    open: BTreeSet<String>,
}

impl<'a> Widths<'a> {
    fn rule(&mut self, name: &str, boundary: u64) -> u64 {
        if let Some(w) = self.known.get(name) {
            return *w;
        }
        // **A rule that reaches itself is taken at nothing for the recursive
        // call** (D1): it can only make the width smaller.
        if !self.open.insert(name.to_string()) {
            return 0;
        }
        let Some(rule) = self.rules.get(name).copied() else {
            self.open.remove(name);
            return 0;
        };
        let w = rule
            .alts
            .iter()
            .map(|alt| self.pattern(&alt.pattern.node, boundary))
            .min()
            .unwrap_or(0);
        self.open.remove(name);
        // Only a width found with nothing open is the rule's own: one found
        // inside a cycle took a recursive call at nothing on the way.
        if self.open.is_empty() {
            self.known.insert(name.to_string(), w);
        }
        w
    }

    fn pattern(&mut self, pattern: &Pattern, boundary: u64) -> u64 {
        match pattern {
            Pattern::Literal(text) => literal_bytes(text),
            Pattern::Seq(parts) => parts.iter().fold(0u64, |sum, p| {
                sum.saturating_add(self.pattern(&p.node, boundary))
            }),
            Pattern::Choice(branches) => branches
                .iter()
                .map(|p| self.pattern(&p.node, boundary))
                .min()
                .unwrap_or(0),
            Pattern::Bind { pat, .. } | Pattern::Group(pat) => self.pattern(&pat.node, boundary),
            Pattern::Repeat { pat, rep } => {
                let times = match rep {
                    Repeat::Star | Repeat::Optional => 0,
                    Repeat::Plus => 1,
                    Repeat::Exactly(m) | Repeat::AtLeast(m) | Repeat::Between(m, _) => {
                        u64::from(*m)
                    }
                };
                match times {
                    0 => 0,
                    _ => self.pattern(&pat.node, boundary).saturating_mul(times),
                }
            }
            Pattern::Cut | Pattern::Fold(_) => 0,
            Pattern::Ref { name, args, .. } => {
                let called = self.parsed.text(*name).to_string();
                if self.rules.contains_key(&called) {
                    return self.rule(&called, boundary);
                }
                match (called.as_str(), args.len()) {
                    // A class matches one character, which takes at least a
                    // byte.
                    ("digit" | "hex_digit" | "any" | "char" | "alpha1" | "multispace1", 0) => 1,
                    ("frame_end", 0) => boundary,
                    // What `p` takes, read as a number or as bytes.
                    ("dec" | "bytes", 1..) => self.pattern(&args[0].node, boundary),
                    // Any other built-in may match nothing, as far as D1
                    // reads it: counting less is never wrong.
                    _ => 0,
                }
            }
        }
    }
}

/// **The least width of `rule` in `grammar`** (ADR-367 D1): the fewest bytes
/// any match of it takes. A `frame_end` inside it is the boundary of the
/// rule's own `@frame`, and nothing where it has none. `None` where the
/// grammar or the rule is not in the program.
pub fn least_width(parsed: &Parsed, grammar: &str, rule: &str) -> Option<u64> {
    parsed
        .program
        .items
        .iter()
        .find_map(|item| match &item.node {
            Item::Grammar(g) if parsed.text(g.name) == grammar => rule_width(parsed, g, rule),
            _ => None,
        })
}

/// [`least_width`] of a rule of a grammar at hand.
fn rule_width(parsed: &Parsed, grammar: &GrammarDef, rule: &str) -> Option<u64> {
    let rules: BTreeMap<String, &GrammarRule> = grammar
        .rules
        .iter()
        .map(|r| (parsed.text(r.name).to_string(), r))
        .collect();
    let element = *rules.get(rule)?;
    let boundary = element
        .frame
        .as_ref()
        .and_then(|f| f.boundary.as_deref())
        .map(literal_bytes)
        .unwrap_or(0);
    let mut widths = Widths {
        parsed,
        rules,
        known: BTreeMap::new(),
        open: BTreeSet::new(),
    };
    Some(widths.rule(rule, boundary))
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
        Extent::Known => Some((LENGTH_BOUND - 1) / u128::from(width.max(1))),
        // ADR-367 D2: a stream has no length, so no step count.
        Extent::Unbounded => None,
    }
}
