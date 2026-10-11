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
//! ledger's, #580 steps 5 and 6. Every accepted program is also compiled with
//! `rustc`: a program the checker accepts and `rustc` refuses is a compiler
//! defect (Part III C.1).

mod common;

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

/// Lower and compile as a library: an accepted program whose Rust `rustc`
/// refuses is a compiler defect (Part III C.1), so every accepted case here
/// goes through it.
fn compiled(purpose: &str, source: &str) {
    let rust = lowered(source);
    let dir = common::scratch_dir(purpose);
    let file = dir.join("lowered.rs");
    std::fs::write(&file, &rust).expect("write the Rust");
    let out = common::compile(
        &file,
        &[
            "--crate-type",
            "lib",
            "--emit=metadata",
            "-o",
            dir.join("lowered.rmeta").to_str().expect("utf-8 path"),
        ],
    );
    assert!(
        out.status.success(),
        "the lowering of {purpose} does not compile:\n{}\n--- the Rust ---\n{rust}",
        String::from_utf8_lossy(&out.stderr),
    );
    let _ = std::fs::remove_dir_all(&dir);
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
    compiled("pick_every_path_is_accepted", source);
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
    compiled("label_world_is_accepted", source);
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
    compiled("loop_split_null_is_accepted", source);
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
    compiled("struct_beside_self_is_accepted", source);
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
    compiled("a_or_literal_is_accepted", source);
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
    compiled("a_or_local_buffer_is_accepted", source);
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
    compiled("literals_only_is_accepted", source);
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
    compiled("recursive_is_accepted", source);
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
    compiled("calls_ledger_is_accepted", source);
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
    compiled("plain_params_is_accepted", source);
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
    compiled("copy_is_none_is_accepted", source);
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
fn a_method_signature_names_the_parameter_not_the_receiver() {
    let rust = lowered(
        "struct Summary { label: ref String }\nimpl Summary {\n    fn pick(ref self, name: ref String) -> ref String sync {\n        return name\n    }\n}\n",
    );
    assert!(rust.contains("name: &'r str) -> &'r str"), "{rust}");
    assert!(!rust.contains("&'r self"), "{rust}");
}

/// Only literals handed back: `'static`, and a caller may change `b` after.
#[test]
fn literals_only_lowers_to_static() {
    let rust = lowered(
        "fn kind(a: ref String, b: ref String) -> ref String {\n    if a.len() > b.len() {\n        return \"long\"\n    }\n    return \"short\"\n}\n",
    );
    assert!(rust.contains("-> &'static str"), "{rust}");
    assert!(!rust.contains("<'p>"), "{rust}");
}

/// A parameter written `String` and only read is a source like any other.
#[test]
fn plain_parameters_lowered_as_views_name_the_source_only() {
    let rust = lowered("fn first_of(a: String, b: String) -> ref String {\n    return ref a\n}\n");
    assert!(
        rust.contains("fn first_of<'r>(a: &'r str, b: &str) -> &'r str"),
        "{rust}"
    );
}

/// The ledger writes the answer, `-> ref(a) String`, and not every view.
#[test]
fn the_ledger_names_the_source() {
    let parsed =
        parse_to_ast("fn pick(a: ref String, b: ref String) -> ref String {\n    return a\n}\n")
            .expect("the source parses");
    let own = Ledger::infer(&parsed);
    let contract = own.functions.get("pick").expect("pick has an entry");
    assert_eq!(contract.borrows, vec!["a".to_string()]);
}

// --- holes the review found: a view that reaches a second parameter by a way the
// walk has to follow, each one NK2302 -------------------------------------------

/// A method that changes a local list may keep its argument: `v.push(b)` makes `v` carry `b`.
#[test]
fn hole_push_into_a_local_list() {
    let source = r#"fn f(a: ref String, b: ref String) -> ref String {
    if a.len() > 9 {
        return a
    }
    let mut v: Vec[ref String] = []
    v.push(b)
    return v[0]
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// Same for a map: `m.insert(1, b)`.
#[test]
fn hole_insert_into_a_local_map() {
    let source = r#"use std::collections
fn f(a: ref String, b: ref String) -> ref String {
    if a.len() > 9 {
        return a
    }
    let mut m: collections::HashMap[i64, ref String] = collections::HashMap()
    m.insert(1, b)
    return m[1] ?? a
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// `fill(v, b)` with a `mut` parameter: the local passed to it carries `b`.
#[test]
fn hole_a_mut_parameter_keeps_the_other_argument() {
    let source = r#"fn fill(mut v: Vec[ref String], b: ref String) {
    v.push(b)
}
fn f(a: ref String, b: ref String) -> ref String {
    if a.len() > 9 {
        return a
    }
    let mut v: Vec[ref String] = []
    fill(v, b)
    return v[0]
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// A copy of a value whose type is not known may hold views: it reaches what the value did.
#[test]
fn hole_clone_of_an_unknown_type() {
    let source = r#"fn f(a: ref String, b: ref String) -> ref String {
    if a.len() > 9 {
        return a
    }
    let t = (b, 1)
    let u = t.clone()
    return u.0
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// `p.clone()` with `p: ref Pair`: the copy is a `Pair`, whose view is still `p`'s.
#[test]
fn hole_clone_of_a_ref_to_a_struct_of_views() {
    let source = r#"struct Pair { l: ref String, n: i64 }
fn f(p: ref Pair, a: ref String) -> ref String {
    if a.len() > 9 {
        return a
    }
    let q = p.clone()
    return q.l
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// `self` by value of a struct of views is a source.
#[test]
fn hole_a_subject_taken_by_value_that_holds_views() {
    let source = r#"struct Pair { l: ref String, n: i64 }
impl Pair {
    fn left(self, a: ref String) -> ref String {
        if a.len() > 9 {
            return a
        }
        return self.l
    }
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// An option written as a view is a parameter and a source.
#[test]
fn hole_a_view_option() {
    let source = r#"fn f(a: ref String; sep: ref String = ",") -> ref String {
    if a.len() > 9 {
        return a
    }
    return sep
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// A chain of reassignments longer than a few rounds of the walk still reaches `b`.
#[test]
fn hole_a_long_reassignment_chain() {
    let source = r#"fn f(a: ref String, b: ref String) -> ref String {
    let mut x1: ref String = a
    let mut x2: ref String = a
    let mut x3: ref String = a
    let mut x4: ref String = a
    let mut x5: ref String = a
    let mut x6: ref String = a
    let mut x7: ref String = a
    let mut x8: ref String = a
    let mut i = 0
    while i < 10 {
        x8 = x7
        x7 = x6
        x6 = x5
        x5 = x4
        x4 = x3
        x3 = x2
        x2 = x1
        x1 = b
        i += 1
    }
    return x8
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// A chain of twelve functions that ends in `b` is followed to its end.
#[test]
fn hole_a_long_call_chain() {
    let source = r#"fn f(a: ref String, b: ref String) -> ref String {
    if a.len() > 9 {
        return a
    }
    return g1(a, b)
}
fn g1(a: ref String, b: ref String) -> ref String {
    return g2(a, b)
}
fn g2(a: ref String, b: ref String) -> ref String {
    return g3(a, b)
}
fn g3(a: ref String, b: ref String) -> ref String {
    return g4(a, b)
}
fn g4(a: ref String, b: ref String) -> ref String {
    return g5(a, b)
}
fn g5(a: ref String, b: ref String) -> ref String {
    return g6(a, b)
}
fn g6(a: ref String, b: ref String) -> ref String {
    return g7(a, b)
}
fn g7(a: ref String, b: ref String) -> ref String {
    return g8(a, b)
}
fn g8(a: ref String, b: ref String) -> ref String {
    return g9(a, b)
}
fn g9(a: ref String, b: ref String) -> ref String {
    return g10(a, b)
}
fn g10(a: ref String, b: ref String) -> ref String {
    return g11(a, b)
}
fn g11(a: ref String, b: ref String) -> ref String {
    return g12(a, b)
}
fn g12(a: ref String, b: ref String) -> ref String {
    return b
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// `let p = mk(b)` hides the parameter `p`: the field read is of the local, not of the declared `Num`.
#[test]
fn hole_a_local_hides_a_parameter_of_the_same_name() {
    let source = r#"struct Num { n: i64 }
struct Txt { n: ref String }
fn mk(x: ref String) -> Txt {
    return Txt { n: x }
}
fn f(p: ref Num, a: ref String, b: ref String) -> ref String {
    if a.len() > 9 {
        return a
    }
    let p = mk(b)
    return p.n
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// `fn() -> ref String` says nothing about whose data the result is, and has nothing to be elided from.
#[test]
fn a_function_type_that_takes_no_view_and_hands_one_back() {
    let source = r#"fn f(g: fn() -> ref String sync, a: ref String) -> ref String {
    if a.len() > 9 {
        return a
    }
    return g()
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// No receiver: the declaration's only view is `x`; a plain `y` the body reads is not part of the declaration.
#[test]
fn a_trait_impl_hands_back_only_what_the_declaration_names() {
    let source = r#"trait Picks {
    fn pick(x: ref String, y: String) -> ref String
}
struct Holder { n: i64 }
impl Picks for Holder {
    fn pick(x: ref String, y: String) -> ref String {
        return ref y
    }
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

/// `b` is moved into the method and ends with it, and nothing keeps it.
#[test]
fn a_view_of_a_parameter_taken_by_value_in_a_method() {
    let source = r#"struct S { name: String }
impl S {
    fn get(ref self, b: String) -> ref String {
        if b.len() > 9 {
            return self.name
        }
        return ref b
    }
}
"#;
    let found = all_findings(source);
    assert!(!found.is_empty(), "accepted, and it should not be");
    assert!(
        found.iter().any(|f| f.message.contains("takes by value")),
        "{found:#?}"
    );
    for finding in &found {
        let shown = nikaia::diagnostics::render_finding(finding, "p.nika", source);
        assert!(!says_a_lifetime(&shown), "{shown}");
    }
}

// --- accepted shapes the review found failing in rustc ------------------------------

/// `t: ref Txt` has a loan and a buffer, and the result is the buffer's:
/// `fn name_of(t: &Txt<'r>) -> &'r str`.
#[test]
fn the_buffer_of_a_struct_parameter_is_the_source() {
    let source = r#"struct Txt { n: ref String }
fn mk(x: ref String) -> Txt {
    return Txt { n: x }
}
fn name_of(t: ref Txt) -> ref String {
    return t.n
}
fn f(b: ref String) -> ref String {
    let t = mk(b)
    return name_of(t)
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
    compiled("the_buffer_of_a_struct_parameter_is_the_source", source);
    let rust = lowered(source);
    assert!(
        rust.contains("fn name_of<'r>(t: &Txt<'r>) -> &'r str"),
        "{rust}"
    );
}

/// A view of a text the body formats, beside a view of a parameter.
#[test]
fn a_formatted_text_beside_a_parameter_is_kept() {
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
    compiled("a_formatted_text_beside_a_parameter_is_kept", source);
}

/// A caller may change what the result does not point into.
#[test]
fn a_caller_may_change_the_argument_the_result_does_not_point_into() {
    let source = r#"fn pick(a: ref String, b: ref String) -> ref String {
    if b.len() > 100 {
        return a
    }
    return a.trim()
}
fn main() {
    let x = " ab ".clone()
    let mut y = "c".clone()
    let r = pick(x, y)
    y = "dd".clone()
    println(r)
    println(y)
}
"#;
    let found = all_findings(source);
    assert!(found.is_empty(), "{found:#?}");
    compiled("a_caller_may_change_the_argument", source);
}

// --- the message: Part III's catalogue row, reproduced -------------------------------

/// The reproduction of the `NK2302` row for a result that points into two
/// parameters: the headline, a caret under each way out named with its
/// parameter, the note and both ways out. No word of the language below.
#[test]
fn the_catalogue_row_for_two_sources_is_reproduced() {
    let source = "fn longest(a: ref String, b: ref String) -> ref String {\n    if a.len() >= b.len() {\n        return a\n    }\n    return b\n}\n";
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(
        shown.contains(
            "error[NK2302]: `longest` hands back a view of `a`'s data here and of `b`'s there, so a caller can't tell whose data it gets."
        ),
        "{shown}"
    );
    assert!(shown.contains("--> p.nika:3:9"), "{shown}");
    assert!(shown.contains("^^^^^^^^ a view of `a`"), "{shown}");
    assert!(shown.contains("-------- a view of `b`"), "{shown}");
    assert!(
        shown.contains("here that's decided only when the program runs."),
        "{shown}"
    );
    assert!(
        shown.contains("Split it, so each part hands back views"),
        "{shown}"
    );
    assert!(
        shown.contains("Or hand back a copy: declare `-> String`"),
        "{shown}"
    );
    assert!(!says_a_lifetime(&shown), "{shown}");
}

/// A declaration with no body names no way out: the caret is the declaration.
#[test]
fn the_declaration_site_variant_names_the_declaration() {
    let source = "trait Picks {\n    fn pick(a: ref String, b: ref String) -> ref String\n}\n";
    let found = result_findings(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    let shown = nikaia::diagnostics::render_finding(&found[0], "p.nika", source);
    assert!(shown.contains("--> p.nika:2:"), "{shown}");
    assert!(shown.contains("is declared without a body"), "{shown}");
    assert!(!says_a_lifetime(&shown), "{shown}");
}

// --- a result type another file declares (B12) -----------------------------------

/// Write a two-file package and read it: `(directory, program)`.
fn package(name: &str, files: &[(&str, &str)]) -> (std::path::PathBuf, nikaia::modules::Program) {
    let dir = common::scratch_dir(&format!("result-source-{name}"));
    for (file, contents) in files {
        std::fs::write(dir.join(file), contents).expect("write the file");
    }
    let program =
        nikaia::modules::Program::read(&dir.join("main.nika")).expect("the package reads");
    (dir, program)
}

/// The result refusals of every file of a package.
fn package_findings(program: &nikaia::modules::Program) -> Vec<Finding> {
    let library = Ledger::parse(STD).expect("std's shipped ledger parses");
    let packages = program.package_names();
    program
        .units
        .iter()
        .flat_map(|unit| {
            check::check_program(&unit.parsed, &program.contracts, &library, &packages).findings
        })
        .filter(|f| f.code == "NK2302")
        .collect()
}

/// Lower the package and compile it: an accepted package whose Rust `rustc`
/// refuses is a compiler defect (Part III C.1).
fn package_compiled(purpose: &str, dir: &std::path::Path, program: &nikaia::modules::Program) {
    let lowered = program
        .emit_reading(Build::default(), &common::reads())
        .expect("the package lowers");
    let file = dir.join("program.rs");
    std::fs::write(&file, &lowered.rust).expect("write the Rust");
    let out = common::compile(
        &file,
        &[
            "--crate-type",
            "bin",
            "-o",
            dir.join("program").to_str().expect("utf-8 path"),
        ],
    );
    assert!(
        out.status.success(),
        "the lowering of {purpose} does not compile:\n{}\n--- the Rust ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        lowered.rust,
    );
}

const HELPER: &str = "pub struct Txt { pub n: ref String }\n\
     pub fn mk(x: ref String) -> Txt {\n    return Txt { n: x }\n}\n";

/// **A struct of views another file declares is judged like one declared here**
/// (B12): a result holding the views of `a` on one way out and of `b` on
/// another is `NK2302`, not a rustc error about a file nobody wrote.
#[test]
fn a_foreign_struct_result_with_two_sources_stays_nk2302() {
    let main = "fn hole(a: ref String, b: ref String) -> Txt {\n    if a.len() > 9 {\n        return Txt { n: a }\n    }\n    return mk(b)\n}\n\
        fn main() {\n    let x = \"hello\".clone()\n    let y = \"hi\".clone()\n    let t = hole(x, y)\n    println(t.n)\n}\n";
    let (dir, program) = package("xfile-two", &[("helper.nika", HELPER), ("main.nika", main)]);
    let found = package_findings(&program);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].message.contains("whose data it gets"),
        "{}",
        found[0].message
    );
    assert!(!says_a_lifetime(&found[0].message), "{}", found[0].message);
    let _ = std::fs::remove_dir_all(dir);
}

/// …and a struct of views of one parameter on every way out is accepted and compiles.
#[test]
fn a_foreign_struct_result_with_one_source_is_accepted() {
    let main = "fn hole(a: ref String, b: ref String) -> Txt {\n    if b.len() > 9 {\n        return mk(a)\n    }\n    return Txt { n: a }\n}\n\
        fn main() {\n    let x = \"hello\".clone()\n    let y = \"hi\".clone()\n    let t = hole(x, y)\n    println(t.n)\n}\n";
    let (dir, program) = package("xfile-one", &[("helper.nika", HELPER), ("main.nika", main)]);
    let found = package_findings(&program);
    assert!(found.is_empty(), "{found:#?}");
    package_compiled("xfile-one", &dir, &program);
    let _ = std::fs::remove_dir_all(dir);
}

/// A struct of another file that holds no view is no source: two view
/// parameters beside it are accepted (the shape of `bt_named`) and compile.
#[test]
fn a_foreign_struct_without_views_is_accepted() {
    let helper = "pub struct Pair { pub n: String, pub k: i64 }\n";
    let main = "fn both(a: ref String, b: ref String) -> Pair {\n    return Pair { n: a.clone(), k: b.len() }\n}\n\
        fn main() {\n    let x = \"hello\".clone()\n    let y = \"hi\".clone()\n    let t = both(x, y)\n    println(t.n)\n}\n";
    let (dir, program) = package(
        "xfile-plain",
        &[("helper.nika", helper), ("main.nika", main)],
    );
    let found = package_findings(&program);
    assert!(found.is_empty(), "{found:#?}");
    package_compiled("xfile-plain", &dir, &program);
    let _ = std::fs::remove_dir_all(dir);
}
