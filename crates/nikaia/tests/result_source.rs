//! **A result points into at most one parameter, and the body says which**
//! ([ADR-283](../../docs/specification/adr/adr-283.md) D25, D26; Part I 6.6;
//! #580).
//!
//! Three parts. The first is what is accepted now that a function with several
//! views is judged by its ways out and not by its signature. The second is the
//! dangle attempts that stay `NK2302`: two parameters on the ways out, in every
//! shape the walk follows a view through. The third is the one behaviour change
//! the owner has not confirmed (`plain_string_longest_is_refused_since_d25`),
//! kept in a test of its own so it is easy to find and to revert.
//!
//! What the lowering writes for the accepted cases (the lifetime on the source
//! only, `'static` for none, the ledger's `ref(a)`) is the emitter's and the
//! ledger's, #580 steps 5 and 6; those tests are `#[ignore]`d until then.

use nikaia::check::{self, Finding};
use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn all_findings(source: &str) -> Vec<Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's shipped ledger parses");
    check::check(&parsed, &own, &library)
        .findings
        .into_iter()
        .filter(|f| f.code == "NK2302")
        .collect()
}

/// The refusals that say a result may point into two parameters: D25's
/// message, D26's for a declaration and for a trait impl.
fn result_findings(source: &str) -> Vec<Finding> {
    all_findings(source)
        .into_iter()
        .filter(|f| {
            f.message.contains("whose data it gets")
                || f.message.contains("implements a trait method")
        })
        .collect()
}

fn lowered(source: &str) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    emit_program(&parsed, Build::default())
        .expect("the source lowers")
        .rust
}

/// A lifetime is a tick and a name (`'a`, `'static`), and Part III C.2 keeps
/// both out of a message: the language below is not spoken to the author.
fn says_a_lifetime(text: &str) -> bool {
    let bytes = text.as_bytes();
    text.to_lowercase().contains("lifetime")
        || bytes.windows(3).any(|w| {
            (w[0] == b' ' || w[0] == b'(' || w[0] == b'<' || w[0] == b'&')
                && w[1] == b'\''
                && w[2].is_ascii_lowercase()
        })
}

// --- accepted ------------------------------------------------------------------

/// A function with two views that hands back `a` (or a view cut from `a`) on every way out points into `a` alone.
#[test]
fn pick_every_path_is_accepted() {
    let source = r#"fn pick(a: ref String, b: ref String) -> ref String {
    if a.len() > 3 {
        return a
    }
    return a.trim()
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// The receiver counts as a source, and an index reads only its base: `world.names[self.at]` points into `world`, and `self` is free again.
#[test]
fn label_world_is_accepted() {
    let source = r#"struct World { names: Vec[String] }
struct Checker { at: i64 }
impl Checker {
    fn label(ref self, world: ref World) -> ref String {
        return world.names[self.at]
    }
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// A `ref String?` built from a loop over `a.split(..)`, with `return null` as the other way out.
#[test]
fn loop_split_null_is_accepted() {
    let source = r#"fn first_long(a: ref String, b: ref String) -> ref String? {
    for w in a.split(" ") {
        if w.len() > 3 {
            return w
        }
    }
    return null
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// A struct literal of the result type pointing into one parameter, beside a `ref self` it only reads a scalar from.
#[test]
fn struct_beside_self_is_accepted() {
    let source = r#"struct Reading { name: ref String, temp: i32 }
struct Factory { n: i32 }
impl Factory {
    fn make(ref self, name: ref String) -> Reading {
        return Reading { name: name, temp: self.n }
    }
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// One way out a parameter, the other a literal: the literal outlives everything.
#[test]
fn a_or_literal_is_accepted() {
    let source = r#"fn name_or_none(a: ref String, b: ref String) -> ref String {
    if a.len() > 3 {
        return a
    }
    return "none"
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// One way out a parameter, the other a view of a buffer the body made (a rustc refusal before D25).
#[test]
fn a_or_local_buffer_is_accepted() {
    let source = r#"fn name_or_made(a: ref String, b: ref String) -> ref String {
    if a.len() > 3 {
        return a
    }
    let s = f"{b}!"
    return ref s
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// Only literals handed back: the result points into no parameter.
#[test]
fn literals_only_is_accepted() {
    let source = r#"fn kind(a: ref String, b: ref String) -> ref String {
    if a.len() > b.len() {
        return "long"
    }
    return "short"
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// A recursive function whose recursive call is handed `a` cut down and `b`: the least fixpoint from no parameter gives `a`.
#[test]
fn recursive_is_accepted() {
    let source = r#"fn skip(a: ref String, b: ref String, n: i64) -> ref String {
    if n == 0 {
        return a
    }
    return skip(a.trim(), b, n - 1)
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// A call to a function whose result points into its first argument reaches only that argument.
#[test]
fn calls_ledger_is_accepted() {
    let source = r#"fn first(a: ref String, b: ref String) -> ref String {
    return a
}
fn wrap(x: ref String, y: ref String) -> ref String {
    return first(x, y)
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// Parameters written `String` that are only read are lowered as views (ADR-283 D24) and count as sources like any other.
#[test]
fn plain_params_is_accepted() {
    let source = r#"fn first_of(a: String, b: String) -> ref String {
    return ref a
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// A copy (`b.clone()`) reaches no parameter.
#[test]
fn copy_is_none_is_accepted() {
    let source = r#"fn first_copy(a: ref String, b: ref String) -> ref String {
    let c = b.clone()
    return a
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// The accepted program of a `ref self` method: the receiver is not reached by
/// the one way out, which hands back the parameter.
#[test]
fn a_method_handing_back_its_parameter_is_accepted() {
    let source = r#"
struct Summary { label: ref String }
impl Summary {
    fn pick(ref self, name: ref String) -> ref String sync {
        return name
    }
}
"#;
    assert!(all_findings(source).is_empty());
}

// --- what stays NK2302 ---------------------------------------------------------

#[test]
fn two_ways_stays_nk2302() {
    let source = r#"fn longest(a: ref String, b: ref String) -> ref String {
    if a.len() >= b.len() {
        return a
    }
    return b
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:3:9"), "{shown}");
    assert!(shown.contains("return a"), "{shown}");
}

#[test]
fn loop_best_stays_nk2302() {
    let source = r#"fn last_piece(a: ref String, b: ref String) -> ref String {
    let mut best = a
    for w in b.split(" ") {
        best = w
    }
    return best
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:6:5"), "{shown}");
    assert!(shown.contains("return best"), "{shown}");
}

#[test]
fn coalesce_stays_nk2302() {
    let source = r#"fn either(opt_a: ref String?, b: ref String) -> ref String {
    return opt_a ?? b
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:2:5"), "{shown}");
    assert!(shown.contains("return opt_a ?? b"), "{shown}");
}

#[test]
fn struct_field_then_a_stays_nk2302() {
    let source = r#"struct Pair { l: ref String }
fn pick_l(a: ref String, b: ref String) -> ref String {
    if a.len() > 2 {
        return a
    }
    let p = Pair { l: b }
    return p.l
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:4:9"), "{shown}");
    assert!(shown.contains("return a"), "{shown}");
}

#[test]
fn lambda_stays_nk2302() {
    let source = r#"fn via_lambda(a: ref String, b: ref String) -> ref String {
    let pick = fn() { b }
    if a.len() > 2 {
        return a
    }
    return pick()
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:4:9"), "{shown}");
    assert!(shown.contains("return a"), "{shown}");
}

#[test]
fn call_either_stays_nk2302() {
    let source = r#"fn either(x: ref String, y: ref String) -> ref String {
    if x.len() > y.len() {
        return x
    }
    return y
}
fn through(a: ref String, b: ref String) -> ref String {
    return either(a, b)
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 2, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[found.len() - 1], "p.nika", source);
    assert!(shown.contains("--> p.nika:8:5"), "{shown}");
    assert!(shown.contains("return either(a, b)"), "{shown}");
}

#[test]
fn recursion_swapped_stays_nk2302() {
    let source = r#"fn swap(a: ref String, b: ref String, n: i64) -> ref String {
    if n == 0 {
        return a
    }
    return swap(b, a, n - 1)
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:3:9"), "{shown}");
    assert!(shown.contains("return a"), "{shown}");
}

#[test]
fn struct_of_both_stays_nk2302() {
    let source = r#"struct Pair { l: ref String, r: ref String }
fn both(a: ref String, b: ref String) -> Pair {
    return Pair { l: a, r: b }
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:3:5"), "{shown}");
    assert!(shown.contains("return Pair"), "{shown}");
}

#[test]
fn self_and_world_stays_nk2302() {
    let source = r#"use std::collections
struct World { default: String }
struct Checker { cache: collections::HashMap[String, String] }
impl Checker {
    fn find(ref self, world: ref World, k: ref String) -> ref String {
        return ref (self.cache[k] ?? world.default)
    }
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:6:9"), "{shown}");
    assert!(shown.contains("return ref (self.cache"), "{shown}");
}

#[test]
fn function_type_stays_nk2302() {
    let source = r#"fn apply(f: fn(ref String, ref String) -> ref String sync) -> i64 {
    return 1
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:1:10"), "{shown}");
    assert!(shown.contains("fn(ref String, ref String)"), "{shown}");
}

#[test]
fn trait_impl_stays_nk2302() {
    let source = r#"trait Picks {
    fn pick(ref self, other: ref String) -> ref String
}
struct Holder { label: String }
impl Picks for Holder {
    fn pick(ref self, other: ref String) -> ref String {
        return other
    }
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:6:5"), "{shown}");
    assert!(shown.contains("fn pick(ref self, other"), "{shown}");
}

#[test]
fn trait_decl_stays_nk2302() {
    let source = r#"trait Chooses {
    fn choose(a: ref String, b: ref String) -> ref String
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:2:5"), "{shown}");
    assert!(shown.contains("fn choose(a"), "{shown}");
}

// --- the one behaviour change the owner has not confirmed -----------------------

/// **OPEN POINT of the review of #580, kept apart so it can be reverted.**
///
/// `fn longest(a: String, b: String) -> ref String` compiled before D25: two
/// parameters lowered as views (ADR-283 D24) shared one lifetime in the
/// emitter, which is the union tethering D25 rejects, while the same function
/// written with `ref` was refused. D25 as written refuses both. The owner has
/// not yet confirmed that a program that compiled is refused; to keep it, delete
/// this test and make D24 parameters count only when written `ref`.
#[test]
fn plain_string_longest_is_refused_since_d25() {
    let source = r#"fn longest(a: String, b: String) -> ref String {
    if a.len() >= b.len() {
        return ref a
    }
    return ref b
}
"#;
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:3:9"), "{shown}");
    assert!(!says_a_lifetime(&shown), "{shown}");
}

// --- the lowering and the ledger: #580 steps 5 and 6 ---------------------------

/// The signature names the lifetime on the source only.
#[test]
#[ignore = "pending #580 step 6: the emitter ties the source parameter alone"]
fn the_lowered_signature_names_the_source_only() {
    let rust = lowered(
        "fn pick(a: ref String, b: ref String) -> ref String {\n    if a.len() > 3 {\n        return a\n    }\n    return a.trim()\n}\n",
    );
    assert!(
        rust.contains("fn pick<'r>(a: &'r str, b: &str) -> &'r str"),
        "{rust}"
    );
}

/// A method hands back its parameter: the lifetime is on the parameter, not
/// on the receiver elision would have taken.
#[test]
#[ignore = "pending #580 step 6: the emitter ties the source parameter alone"]
fn a_method_signature_names_the_parameter_not_the_receiver() {
    let rust = lowered(
        "struct Summary { label: ref String }\nimpl Summary {\n    fn pick(ref self, name: ref String) -> ref String sync {\n        return name\n    }\n}\n",
    );
    assert!(rust.contains("name: &'r str) -> &'r str"), "{rust}");
    assert!(!rust.contains("&'r self"), "{rust}");
}

/// Only literals handed back: `'static`, and a caller may change `b` after.
#[test]
#[ignore = "pending #580 step 6: the emitter writes 'static for an empty source set"]
fn literals_only_lowers_to_static() {
    let rust = lowered(
        "fn kind(a: ref String, b: ref String) -> ref String {\n    if a.len() > b.len() {\n        return \"long\"\n    }\n    return \"short\"\n}\n",
    );
    assert!(rust.contains("-> &'static str"), "{rust}");
    assert!(!rust.contains("<'p>"), "{rust}");
}

/// A parameter written `String` and only read is a source like any other.
#[test]
#[ignore = "pending #580 step 6: the emitter drops the shared 'p"]
fn plain_parameters_lowered_as_views_name_the_source_only() {
    let rust = lowered("fn first_of(a: String, b: String) -> ref String {\n    return ref a\n}\n");
    assert!(
        rust.contains("fn first_of<'r>(a: &'r str, b: &str) -> &'r str"),
        "{rust}"
    );
}

/// The ledger writes the answer, `-> ref(a) String`, and not every view.
#[test]
#[ignore = "pending #580 step 5: the ledger writes the inferred source"]
fn the_ledger_names_the_source() {
    let parsed =
        parse_to_ast("fn pick(a: ref String, b: ref String) -> ref String {\n    return a\n}\n")
            .expect("the source parses");
    let own = Ledger::infer(&parsed);
    let contract = own.functions.get("pick").expect("pick has an entry");
    assert_eq!(contract.borrows, vec!["a".to_string()]);
}
