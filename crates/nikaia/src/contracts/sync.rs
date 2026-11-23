#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};
// crates/nikaia/src/contracts/sync.rs
//
// Part II 12.1: a `sync` function may only call `sync` functions.
//
// Two analyses of one rule, running in **opposite directions**, and keeping
// them apart is the whole design (ADR-288).
//
// `check` verifies an assertion. Someone wrote `sync`, and this reports the
// calls that contradict it (`NK2202`). It is conservative in the **permissive**
// direction: with no receiver types, `a.method()` cannot be looked up, so it is
// not an error. What *can* be looked up is a call by name - a function in this
// unit, or a path like `io::read_to_string` into a library's ledger - and that
// is where the rule earns its keep, because `fs::` and `io::` are named rather
// than called on a receiver. The check never rejects a program the rule allows,
// and does not yet catch every program the rule forbids. An unchecked promise
// catches nothing at all, so that is worth having and worth saying.
//
// `infer` makes a claim, and therefore runs the other way. It writes `sync`
// into the ledger for a function nobody annotated, and that entry is **shipped**
// (Part III 13.5): a consumer reads it and puts the function inside `access`.
// So it is conservative in the **restrictive** direction - a call it cannot
// resolve is a call it cannot vouch for, and the function does not get the
// promise. The ledger settled this polarity once already, for provenance: "an
// analysis that fails open is a vulnerability generator". Inferring `sync` from
// a body full of calls one cannot see would be exactly that.
//
// **Why infer at all.** Before this, `sync` was opt-in, so almost nothing was
// `sync`, so `access`, `access_all`, `par_iter` and the panic hook - everything
// Part II 12.2 makes safe by demanding a `sync` lambda - could call almost
// nothing. The restrictive side of the language was the unusable one, and the
// way out was to annotate a chain of pure helpers by hand. Now a body that
// provably cannot pause says so on its own, and `sync` in the source becomes an
// **assertion you write where you want it held**, checked against the body,
// rather than a mode you have to enter.

use std::collections::BTreeMap;

use crate::ast::Expr;
use crate::parser::Parsed;

use crate::check::MethodCalls;

use super::Ledger;

/// One call that a `sync` function may not make, declared in Nikaia
/// (`nikaia-std/src/compiler/sync.nika`): the statement it is in, the caller and
/// what it promised, the callee as the ledger names it, whether a library
/// answered, whether the callee never pauses and does not promise it
/// ([ADR-288](../../../docs/specification/adr/adr-288.md) D28), and whether it
/// is a construct rather than a function
/// ([ADR-292](../../../docs/specification/adr/adr-292.md) D16).
pub use nikaia_std::compiler::sync::Violation;

/// Every call a `sync` function makes that the ledgers say can pause.
///
/// **The walk is Nikaia** (`compiler/sync.nika`'s `sync_violations`, #125);
/// this hands it the unit's view and the holes the emitter reads.
pub fn check(parsed: &Parsed, own: &Ledger, library: &Ledger) -> Vec<Violation> {
    nikaia_std::compiler::sync::sync_violations(
        &crate::check::parsed_view_of(parsed),
        own,
        library,
        &mut |expr: &Expr| crate::emit::literal_expressions(parsed, expr),
    )
}

/// Give every function in the ledger the `sync` its body earns: the walk,
/// the fixpoint (callees first, ADR-100 D7), the column written back and the
/// notes are `compiler/sync.nika`'s `sync_infer` (#125). Only ever *adds*
/// [`Sync::Inferred`](super::Sync): an assertion in the source is left as it
/// was written, so that `NK2202` still has something to contradict. The
/// fixpoint is a greatest one, so mutual recursion between pure functions
/// keeps the claim.
///
/// `resolved` is the type checker's answer to the one question the walk
/// cannot ask: what a method call goes to (ADR-288). Handed in, so that the
/// compiler keeps one type checker.
///
/// **Hands back the functions that pause only where their lambdas do**
/// ([ADR-288](../../../docs/specification/adr/adr-288.md) D29), and why each
/// function that pauses does (D32), for the build to say.
pub fn infer(
    ledger: &mut Ledger,
    units: &[&Parsed],
    library: &Ledger,
    resolved: &BTreeMap<String, MethodCalls>,
) -> Noted {
    let views: Vec<_> = units
        .iter()
        .map(|parsed| crate::check::parsed_view_of(parsed))
        .collect();
    let noted = nikaia_std::compiler::sync::sync_infer(
        ledger,
        &views,
        library,
        resolved,
        &mut |unit: i64, expr: &Expr| crate::emit::literal_expressions(units[unit as usize], expr),
    );
    Noted {
        by_code: noted.by_code,
        why: noted.why,
        ..Noted::default()
    }
}

/// What [`infer`] learns besides the ledger's own column, for the build to say
/// ([ADR-288](../../../docs/specification/adr/adr-288.md) D29, D32).
#[derive(Debug)]
pub struct Noted {
    /// The functions that pause only where their lambdas do, with the code
    /// parameters that decide (D2).
    pub by_code: BTreeMap<String, Vec<String>>,
    /// Why each function that can pause does (D5).
    pub why: BTreeMap<String, Pause>,
    /// **What the impls of trait methods keep on their own evidence**
    /// ([ADR-277](../../../docs/specification/adr/adr-277.md) D17): `NK1264`
    /// and the keepers `NK1265` names.
    pub column: super::keeps::Column,
}

impl Default for Noted {
    fn default() -> Self {
        Noted {
            by_code: BTreeMap::new(),
            why: BTreeMap::new(),
            // The lowered column has no `Default`: nothing kept, nothing beyond.
            column: super::keeps::Column {
                beyond: Vec::new(),
                keepers: BTreeMap::new(),
            },
        }
    }
}

/// **The way from a function to the statement that makes it pause**, in
/// Nikaia (`tools/modules.nika`, which says it, ADR-294, #125).
pub use nikaia_std::compiler::modules::ModulesPause as Pause;

/// What one expression tells either analysis, where it is a call at all.
///
/// One resolution rule, written once. The check and the inference disagree
/// about what to *do* with `Opaque` - the first shrugs, the second refuses -
/// and that disagreement is the design. Having them disagree about what a call
/// even resolves to would just be a bug waiting to happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reached {
    /// A function this package declares, by the name the ledger records it under.
    Own(String),
    /// A function in a library, and what that library's ledger says about it.
    Library { key: String, sync: bool },
    /// A method call. Neither analysis here can resolve one: `stats.add(5)`
    /// names `add` and says nothing about what `stats` is.
    ///
    /// The **type checker** can, and does (ADR-288). So this is not "unknown"
    /// but "asked elsewhere", and the two callers of `reached` take it
    /// differently: the inference merges in the checker's answer per function,
    /// and the check looks the resolved name up the same way it looks up any
    /// other. Collapsing this into `Opaque` was what threw the answer away.
    Method,
    /// A call whose target this compiler cannot name and nobody else can
    /// either: a name no ledger knows.
    ///
    /// Also everything that is not a plain call but still *runs* something -
    /// `spawn`, a `dsl` - because a body containing one is not the pure CPU
    /// task Part II 12.1 describes, whatever the thing it runs turns out to do.
    ///
    /// `Some(name)` where the source wrote one and no ledger knew it, `None`
    /// for a construct that has no callee to name. Both block, and the name is
    /// carried only so the diagnostic can say which call it was about.
    Opaque(Option<String>),
}

/// What a call resolves to, by the same rule for every analysis:
/// `tools/calls.nika`'s `callee_of` (#125). This compiler's walk hands it each
/// expression and reads the answer back as its own enum.
///
/// `None` means the expression is not a call at all - or a variant of a type,
/// which builds a value and runs no body - the one case no analysis has
/// anything to say about.
pub(crate) fn reached(
    parsed: &Parsed,
    expr: &Expr,
    own: &Ledger,
    library: &Ledger,
) -> Option<Reached> {
    use nikaia_std::compiler::calls::{Callee, callee_of};
    Some(
        match callee_of(&parsed.interner, expr, own, library, &parsed.program.items)? {
            Callee::Own(name) => Reached::Own(name),
            Callee::Library { key, never_pauses } => Reached::Library {
                key,
                sync: never_pauses,
            },
            Callee::Method => Reached::Method,
            Callee::Opaque(name) => Reached::Opaque(name),
        },
    )
}
