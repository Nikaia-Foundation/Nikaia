// crates/nikaia/src/contracts/keeps.rs
//
// Which parameters a body **keeps** ([ADR-094](../../../docs/specification/adr/adr-094.md) D2).
//
// The question the caller is asked today and should not be: `page(entries)` or
// `page(&entries)`. Whether an argument is lent or handed over is a fact about
// the *callee's body*, and the caller repeating it is 42 `&` in 913 non-comment
// lines of `examples/` saying what the signature had already said — plus a
// `rustc` error about a moved value wherever one is left out.
//
// *Keeps* means the value outlives the call: stored into a struct, assigned
// into a place, handed back by value, given to a task, or passed to a callee
// whose own parameter keeps it. Everything else is a read, and a read can be
// lent.
//
// **Fail closed, and here that means *keeps*.** A use this walk cannot account
// for counts as keeping, because the two wrong answers are not symmetric.
// Saying *kept* of a value that is only read costs a caller an owned argument —
// which is where every caller already is, so it costs nothing anybody has. Saying
// *lent* of a value the body stores emits a `&T` parameter whose body moves it,
// and that is `rustc`'s error about a file nobody wrote (Part III C.1). Same
// polarity as `sync` ([ADR-288](../../../docs/specification/adr/adr-288.md)) and
// as [ADR-010](../../../docs/specification/adr/adr-010.md) D1, pointing the
// other way because the claim points the other way.
//
// **Three readers act on it** (`lends` below): the emitter writing a
// declaration, the emitter writing a call, and the checker refusing a `&`
// somebody wrote. ADR-094 §5's first step had none, on purpose, so that the
// answer could be diffed against the corpus before one call site changed.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, Item};
use crate::parser::Parsed;

use super::Ledger;

/// Whether a callee **lends** the parameter at `at`, so that the compiler
/// writes the reference and the caller does not
/// ([ADR-094](../../../docs/specification/adr/adr-094.md) D1, D2).
///
/// One answer with three readers — the emitter writing the declaration, the
/// emitter writing the call, and the checker refusing a `&` somebody wrote —
/// because two of the three disagreeing is a `&&T` or a moved value, and both
/// are `rustc`'s words about a file nobody wrote (Part III C.1).
///
/// **The position has to exist and not be the receiver** — `&self` and `self`
/// are D6's and are untouched — and then one of two things has to hold.
///
/// *The type is written as a view.* `&str` and `&Vec[Row]` in a declaration
/// are the assertion D2 keeps: the parameter is a view whatever the body does,
/// so the argument gains a `&` at the call and the declaration is left exactly
/// as it was written. `keeps` is not consulted, because it has nothing to say
/// about a parameter whose kind the author wrote down.
///
/// *Or the value **moves** and the callee does not **keep** it.* That is the
/// inferred half, and both halves of the condition are load-bearing. A number,
/// a `bool`, a `char` and a hull are copied, so lending them buys nothing and
/// costs a dereference at every use; a type this compiler cannot name — `?`, a
/// type variable, a function type — is the absence of an answer, and a
/// reference written on a guess is a guess the language below reports. The
/// `keeps` clause is the column itself.
pub fn lends(contract: &super::FnContract, at: usize) -> bool {
    lends_in(contract, at, &[])
}

/// [`lends`], where a type a ledger says **copies** is copied rather than lent
/// ([ADR-294](../../../docs/specification/adr/adr-294.md) D9.1, the reading):
/// `winnow_grammar::Symbol` is one `u32`, and a `&Symbol` handed on to a
/// described call that takes a `Symbol` is `rustc`'s mismatch about a file
/// nobody wrote. Only a ledger's `copies = true` answers here; a type this
/// program declares is still lent, and reading its copy is the same step for
/// the types a unit declares.
///
/// The rule is `tools/lends.nika`'s (ADR-294, #125), with the three below.
pub fn lends_in(contract: &super::FnContract, at: usize, copying: &[&super::Ledger]) -> bool {
    nikaia_std::tools::lends::lends_in(contract, at as i64, copying)
}

/// [`lends_in`] for a contract of this program's own: a method's function
/// value is lent too.
pub fn lends_in_own(contract: &super::FnContract, at: usize, copying: &[&super::Ledger]) -> bool {
    nikaia_std::tools::lends::lends_in_own(contract, at as i64, copying)
}

/// Whether a ledger says values of `ty` are copies.
pub fn a_ledger_copies(ty: &super::ty::Ty, copying: &[&super::Ledger]) -> bool {
    nikaia_std::tools::lends::a_ledger_copies(ty, copying)
}

/// Whether a value of this type is **moved** when it is handed on, rather than
/// copied — and whether this compiler can say so at all: `Unknown`, a type
/// variable and a function type's absence of an answer are **no**, because what
/// hangs on it is a reference this compiler would *write*
/// ([ADR-312](../../../docs/specification/adr/adr-312.md) D1, ADR-302 D6,
/// ADR-150 D1, ADR-152 D2, ADR-277 D6).
pub fn moves(ty: &super::ty::Ty) -> bool {
    nikaia_std::tools::lends::moves(ty)
}

pub use nikaia_std::tools::keeps::{KeepsColumn as Column, KeepsKeeper as Keeper};
use nikaia_std::tools::keeps::{KeepsHost, KeepsIndexes};

/// Give every function in the ledger the `keeps` its body earns: the walk, the
/// trait edges and the least fixpoint are Nikaia (`tools/keeps.nika`,
/// ADR-294, #125); this builds what they are handed.
pub fn infer(
    ledger: &mut Ledger,
    units: &[&Parsed],
    library: &Ledger,
    resolved: &BTreeMap<String, crate::check::MethodCalls>,
) -> Column {
    let views: Vec<_> = units
        .iter()
        .map(|parsed| crate::check::parsed_view_of(parsed))
        .collect();
    let own_methods = nikaia_std::tools::ty::method_index(ledger);
    let library_methods = nikaia_std::tools::ty::method_index(library);
    let host = Host {
        units,
        own_methods: &own_methods,
        library_methods: &library_methods,
        resolved,
    };
    nikaia_std::tools::keeps::keeps_infer(
        ledger,
        &views,
        library,
        resolved,
        &KeepsIndexes {
            own: &own_methods,
            library: &library_methods,
        },
        &host,
    )
}

/// What the Nikaia walk asks of the compiler: a literal's holes, and the
/// statement that keeps a parameter, which needs the unit's own tokens.
struct Host<'a> {
    units: &'a [&'a Parsed],
    own_methods: &'a nikaia_std::tools::ty::MethodIndex,
    library_methods: &'a nikaia_std::tools::ty::MethodIndex,
    resolved: &'a BTreeMap<String, crate::check::MethodCalls>,
}

impl KeepsHost for Host<'_> {
    fn holes(&self, unit: i64, expr: &Expr) -> Vec<Expr> {
        crate::emit::literal_expressions(self.units[unit as usize], expr)
    }

    fn keeping_statement(
        &self,
        unit: i64,
        method: &crate::ast::Spanned<Item>,
        target: String,
        parameter: String,
        own_kept: &BTreeMap<String, BTreeSet<String>>,
        own: &Ledger,
        library: &Ledger,
    ) -> crate::ast::Span {
        keeping_statement(
            self.units[unit as usize],
            method,
            &target,
            &parameter,
            own_kept,
            (own, library),
            (self.own_methods, self.library_methods),
            self.resolved,
        )
    }
}

/// **The statement that keeps `parameter`**: the first prefix of the impl's
/// body whose walk keeps it, on its own evidence (`own_kept` answers the
/// callees); the method itself where none does.
#[allow(clippy::too_many_arguments)]
fn keeping_statement(
    parsed: &Parsed,
    method: &crate::ast::Spanned<Item>,
    target: &str,
    parameter: &str,
    own_kept: &BTreeMap<String, BTreeSet<String>>,
    (ledger, library): (&Ledger, &Ledger),
    (own_methods, library_methods): (
        &nikaia_std::tools::ty::MethodIndex,
        &nikaia_std::tools::ty::MethodIndex,
    ),
    resolved: &BTreeMap<String, crate::check::MethodCalls>,
) -> crate::ast::Span {
    let Item::Fn { body, .. } = &method.node else {
        return method.span;
    };
    let view = crate::check::parsed_view_of(parsed);
    for end in 1..=body.stmts.len() {
        let mut prefix = method.node.clone();
        if let Item::Fn { body, .. } = &mut prefix {
            body.stmts.truncate(end);
        }
        let Some(found) = nikaia_std::tools::keeps::keeps_uses_of(
            &view,
            &prefix,
            &Some(target.to_string()),
            ledger,
            library,
            &KeepsIndexes {
                own: own_methods,
                library: library_methods,
            },
            resolved,
            &|expr: &Expr| crate::emit::literal_expressions(parsed, expr),
        ) else {
            continue;
        };
        let uses = found.uses;
        if uses.kept.contains(parameter)
            || uses.passed.iter().any(|passed| {
                passed.parameter == parameter
                    && nikaia_std::tools::lends::keeps_its(
                        &passed.callee,
                        passed.at,
                        own_kept,
                        ledger,
                        library,
                    )
            })
        {
            return body.stmts[end - 1].span;
        }
    }
    method.span
}
