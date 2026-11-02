// crates/nikaia/src/type_rules.rs
//
// **A type's rule, an `assert` after a struct's fields**
// ([ADR-369](../../../docs/specification/adr/adr-369.md)), written out right
// after parsing as the `assert`s it means. The rewrite is Nikaia
// (`crates/nikaia-std/src/tools/type_rules.nika`, ADR-294, #125, R-3): every
// literal of the type checked where it is built (D2), a method that changes
// its receiver checking the rule where it leaves (D2), and the rule given at
// each function's entry (D3). This module is the adapter: it hands the parsed
// tree to the lowered functions, raises their refusal as `NK1288`, and reads a
// rule again in another file with the Rust parser ([`across_files`]).

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, NodeId, Span};
use crate::parser::Parsed;
use nikaia_std::tools::type_rules::{self as tr, TrDeclared, TrRuled};

type Ruled = TrRuled;

/// Writes every rule of the file's structs out as its `assert`s; `NK1288`
/// for a rule that is not one.
pub(crate) fn expand(parsed: &mut Parsed) -> anyhow::Result<()> {
    let interner = parsed.interner.clone();
    let spans = parsed.expr_spans.clone();
    let refusal = tr::tr_expand(
        &interner,
        &|s| interner.intern_string(s),
        &renew,
        &mut parsed.program.items,
        &spans,
    );
    match refusal {
        Some(refusal) => Err(refused(refusal.span, refusal.head, &refusal.help)),
        None => Ok(()),
    }
}

fn refused(span: Span, head: String, help: &str) -> anyhow::Error {
    let finding = crate::check::Finding {
        severity: crate::check::Severity::Error,
        span,
        code: "NK1288".to_string(),
        message: head.clone(),
        notes: Vec::new(),
        help: Some(help.to_string()),
        labels: vec![crate::check::Label {
            span,
            word: String::new(),
            text: String::new(),
            main: true,
        }],
    };
    crate::diagnostics::refuse_finding(finding, format!("NK1288: {head} {help}"))
}

/// An expression copied with a new id.
fn renew(e: &Expr) -> Expr {
    let mut copy = e.clone();
    crate::ast::set_id(&mut copy, NodeId::fresh());
    copy
}

/// **A package of several files keeps a rule as a fact only where no other
/// file can break it** (D3). The rules are written out one file at a time
/// ([`expand`]), so here a literal of a type another file declares is checked
/// as one in the type's own file is (D2), its rules read again in the file
/// that builds it; a method of the type that changes `self` in another file,
/// and a field of it written, set by a `with` or handed on there, each as
/// `tr_changed_outside` reads them, take the type's rule out of the facts at
/// every entry of the package.
///
/// **Nor where another package can break it.** A package is lowered without
/// the code of the packages that use it: a `pub` field a rule reads may be
/// written there, and a literal of a `pub` type whose every field is `pub`
/// (or has a default) may be built there, neither checked. Such a type's
/// rule is no fact anywhere in its package; what is lost is a proof, never a
/// check.
pub(crate) fn across_files(files: &mut [(&mut Parsed, &str)]) {
    let declared: Vec<BTreeMap<String, TrDeclared>> = files
        .iter()
        .map(|(file, source)| {
            tr::tr_declared_rules(&file.interner, &file.program.items, &|span| {
                source
                    .get(span.start as usize..span.end as usize)
                    .unwrap_or_default()
                    .to_string()
            })
        })
        .collect();
    if declared.iter().all(BTreeMap::is_empty) {
        return;
    }
    let mut changed: BTreeSet<String> = declared
        .iter()
        .flat_map(|types| {
            types
                .iter()
                .filter(|(_, d)| d.open)
                .map(|(name, _)| name.clone())
        })
        .collect();
    for (at, (file, _)) in files.iter_mut().enumerate() {
        let foreign: BTreeMap<String, &TrDeclared> = declared
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != at)
            .flat_map(|(_, types)| types.iter().map(|(name, d)| (name.clone(), d)))
            .collect();
        if foreign.is_empty() {
            continue;
        }
        let interner = file.interner.clone();
        let fields: BTreeMap<String, BTreeSet<String>> = foreign
            .iter()
            .map(|(name, d)| (name.clone(), d.fields.clone()))
            .collect();
        let ruled: BTreeMap<String, Ruled> = foreign
            .iter()
            .filter_map(|(name, d)| Some((name.clone(), read_again(file, name, d)?)))
            .collect();
        changed.extend(tr::tr_changed_outside(
            &interner,
            &fields,
            &file.program.items,
        ));
        changed.extend(tr::tr_built_elsewhere(
            &interner,
            &fields,
            &ruled,
            &file.program.items,
        ));
        if ruled.is_empty() {
            continue;
        }
        let spans = file.expr_spans.clone();
        tr::tr_check_literals_in(
            &interner,
            &|s| interner.intern_string(s),
            &renew,
            &ruled,
            &spans,
            &mut file.program.items,
        );
    }
    if changed.is_empty() {
        return;
    }
    for (file, _) in files.iter_mut() {
        let interner = file.interner.clone();
        tr::tr_drop_given(
            &interner,
            &|s| interner.intern_string(s),
            &renew,
            &changed,
            &mut file.program.items,
        );
    }
}

/// A type's rules read again in `file`, with its words: `None` where one
/// does not read as a rule, and a literal of the type is then not checked.
fn read_again(file: &Parsed, type_name: &str, declared: &TrDeclared) -> Option<Ruled> {
    let interner = &file.interner;
    let mut rules = Vec::new();
    for text in &declared.rules {
        let rule = file.read_expression(text).ok()?;
        rules.push(tr::tr_a_rule(interner, &rule, Span::new(0, 0))?);
    }
    Some(Ruled {
        binding: interner.intern_string(&tr::tr_binding_of(type_name, &declared.fields)),
        fields: declared.fields.clone(),
        rules,
    })
}
