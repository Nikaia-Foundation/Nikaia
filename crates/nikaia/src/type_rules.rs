// crates/nikaia/src/type_rules.rs
//
// **A type's rule, an `assert` after a struct's fields**
// ([ADR-369](../../../docs/specification/adr/adr-369.md)), written out right
// after parsing as the `assert`s it means, so the checker, the prover, the
// bounds walk and the lowering read them as any other claim:
//
// * every literal of the type is checked where it is built (D2): `Piece {
//   start: a, end: b }` is `{ let piece = Piece { … }; assert(0 <=
//   piece.start && …; message: …); piece }`, proved where the compiler can;
// * a method that changes its receiver (`ref mut self`, `mut self`) checks
//   the rule over `self` before each `return` and each `throw`, and at its end
//   (D2): inside, it may break the rule for a while;
// * at the entry of every function, each parameter of the type and `self` in
//   its methods hold the rule (D3): an `assert` marked with an option no
//   program can write ([`GIVEN`]), which the prover and the bounds walk take
//   as a fact and nothing checks.
//
// A rule is one `assert(claim)` or `assert(claim; message: …)` whose claim
// reads the fields by name, paths and pure methods from them, `comptime`
// values and literals; anything else is `NK1288`.

use std::collections::{BTreeMap, BTreeSet};

use winnow_grammar::{InternerContext, Symbol};

use crate::ast::{
    Block, ConfigArg, Expr, FnArg, Item, NodeId, Receiver, Span, Spanned, Stmt, Type,
};
use crate::parser::Parsed;
use nikaia_std::tools::type_rules::{self as tr, TrDeclared, TrMethods, TrRuled};

/// The option that marks a rule's `assert` as given rather than claimed: a
/// fact at a function's entry (ADR-369 D3). A name with spaces, which no
/// program can write.
pub const GIVEN: &str = "rule of the type";

type Ruled = TrRuled;

/// Writes every rule of the file's structs out as its `assert`s; `NK1288`
/// for a rule that is not one.
pub(crate) fn expand(parsed: &mut Parsed) -> anyhow::Result<()> {
    let interner = parsed.interner.clone();
    let found = tr::tr_collect(
        &interner,
        &|s| interner.intern_string(s),
        &renew,
        &parsed.program.items,
    );
    if let Some(refusal) = found.refusal {
        return Err(refused(refusal.span, refusal.head, &refusal.help));
    }
    let ruled = found.ruled;
    if ruled.is_empty() {
        return Ok(());
    }
    let spans = parsed.expr_spans.clone();
    let mut items = std::mem::take(&mut parsed.program.items);
    let fields: BTreeMap<String, BTreeSet<String>> = ruled
        .iter()
        .map(|(name, r)| (name.clone(), r.fields.clone()))
        .collect();
    let mut changed = tr::tr_changed_outside(&interner, &fields, &items);
    changed.extend(tr::tr_fails_changed(&interner, &ruled, &items));
    // Each type's methods in this file, and of them those that change `self`.
    let own = tr::tr_methods_of(&interner, &items);
    for item in &mut items {
        match &mut item.node {
            Item::Impl {
                target, methods, ..
            } => {
                let owner = interner.resolve(target.name).to_string();
                for method in methods {
                    let span = method.span;
                    rewrite_fn(
                        &interner,
                        &ruled,
                        &changed,
                        &own,
                        &spans,
                        &mut method.node,
                        Some(&owner),
                        span,
                    );
                }
            }
            Item::Fn { .. } | Item::Test { .. } | Item::Bench { .. } => {
                let span = item.span;
                rewrite_fn(
                    &interner,
                    &ruled,
                    &changed,
                    &own,
                    &spans,
                    &mut item.node,
                    None,
                    span,
                );
            }
            _ => {}
        }
    }
    parsed.program.items = items;
    Ok(())
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

/// A base read once more: the same name or path, a node of its own.
fn renumbered(base: &Expr) -> Expr {
    tr::tr_renumbered(base, &renew)
}

/// `assert(rule over base; message: …)` for each rule of `ruled`, standing
/// at `span`; given rather than claimed where `given` says so.
fn asserts(
    interner: &InternerContext,
    ruled: &Ruled,
    type_name: &str,
    base: &Expr,
    span: Span,
    given: bool,
) -> Vec<Spanned<Stmt>> {
    tr::tr_asserts(
        interner,
        &|s| interner.intern_string(s),
        &renew,
        ruled,
        type_name,
        base,
        &span,
        given,
    )
}

/// [`asserts`], given where `given` says so and with its `message:` where
/// `said` does: a rule a method keeps ([`tr::tr_kept_by`]) is both, so that it
/// is a check again where another file breaks the rule ([`drop_given_in`]).
fn asserts_as(
    interner: &InternerContext,
    ruled: &Ruled,
    type_name: &str,
    base: &Expr,
    span: Span,
    given: bool,
    said: bool,
) -> Vec<Spanned<Stmt>> {
    tr::tr_asserts_as(
        interner,
        &|s| interner.intern_string(s),
        &renew,
        ruled,
        type_name,
        base,
        &span,
        given,
        said,
    )
}

/// The name a type is written by: `Piece`, `ref Piece`, `mut Piece`.
fn named(interner: &InternerContext, ty: &Type) -> String {
    interner.resolve(ty.name).to_string()
}

#[allow(clippy::too_many_arguments)]
fn rewrite_fn(
    interner: &InternerContext,
    ruled: &BTreeMap<String, Ruled>,
    changed: &BTreeSet<String>,
    own: &BTreeMap<String, TrMethods>,
    spans: &BTreeMap<u32, Span>,
    item: &mut Item,
    owner: Option<&str>,
    span: Span,
) {
    let (receiver, args, ret_type, body): (Option<Receiver>, Vec<FnArg>, bool, &mut Block) =
        match item {
            Item::Fn {
                receiver,
                args,
                ret_type,
                body,
                ..
            } => (*receiver, args.clone(), ret_type.is_some(), body),
            Item::Test { body, .. } | Item::Bench { body, .. } => (None, Vec::new(), false, body),
            _ => return,
        };
    check_literals(interner, ruled, spans, body, span);
    let self_name = interner.intern_string("self");
    let self_rules = owner.and_then(|o| ruled.get(o).map(|r| (o, r)));
    let no_methods = TrMethods {
        all: BTreeSet::new(),
        changing: BTreeSet::new(),
    };
    let methods = owner.and_then(|o| own.get(o)).unwrap_or(&no_methods);
    // **A rule the method cannot break is not checked in it** ([`kept_by`]):
    // given at each point instead, with its message for where another file
    // breaks it.
    let mut kept: Option<Ruled> = None;
    let mut checked: Option<Ruled> = None;
    if let (Some((type_name, rules)), Some(how)) = (self_rules, &receiver)
        && how.is_mut
    {
        let taken: Vec<bool> = rules
            .rules
            .iter()
            .map(|rule| {
                !changed.contains(type_name)
                    && tr::tr_kept_by(interner, &rule.claim, &rules.fields, &methods.all, body)
            })
            .collect();
        let others: Vec<bool> = taken.iter().map(|t| !t).collect();
        kept = Some(tr::tr_part(rules, &taken));
        checked = Some(tr::tr_part(rules, &others));
    }
    if let (Some((type_name, rules)), Some(_)) = (self_rules, &receiver)
        && !changed.contains(type_name)
    {
        given_after_calls(interner, rules, type_name, &methods.changing, body);
    }
    // **A method that changes its receiver keeps the rule when it leaves**.
    if let (Some((type_name, _)), Some(how), Some(kept), Some(checked)) =
        (self_rules, &receiver, &kept, &checked)
        && how.is_mut
    {
        let base = Expr::Variable(self_name, NodeId::fresh());
        let asserts = |_: &InternerContext, _: &Ruled, _: &str, base: &Expr, at: Span, _: bool| {
            let mut out = asserts_as(interner, checked, type_name, base, at, false, true);
            out.extend(asserts_as(interner, kept, type_name, base, at, true, true));
            out
        };
        let rules = checked;
        let mut in_lambdas = BTreeSet::new();
        crate::text_tiers::visit_block_mut(
            body,
            &mut |expr| {
                if let Expr::Closure { body, .. } = expr {
                    crate::text_tiers::visit_block_mut(
                        body,
                        &mut |inner| {
                            if let Expr::Return(_, id) | Expr::Throw(_, id) = inner {
                                in_lambdas.insert(id.get());
                            }
                        },
                        &mut |_, _| {},
                    );
                }
            },
            &mut |_, _| {},
        );
        crate::text_tiers::visit_block_mut(
            body,
            &mut |expr| {
                // **`self` handed on holds the rule**: the method or function
                // it reaches reads the rule as a fact at its entry (D3), so a
                // rule broken for a while is checked before the call.
                let hands_self = match &*expr {
                    Expr::MethodCall { receiver, .. } => is_self(interner, receiver),
                    Expr::Call { args, .. } => args.iter().any(|a| is_self(interner, a)),
                    _ => false,
                };
                if hands_self {
                    let at = spans.get(&expr_id(expr)).copied().unwrap_or(span);
                    let call = std::mem::replace(expr, Expr::LitNull(NodeId::fresh()));
                    let mut stmts = asserts(interner, rules, type_name, &base, at, false);
                    stmts.push(Spanned::new(Stmt::Expr(call), at));
                    *expr = Expr::Block(Block { stmts }, NodeId::fresh());
                    return;
                }
                let (Expr::Return(_, id) | Expr::Throw(_, id)) = expr else {
                    return;
                };
                if in_lambdas.contains(&id.get()) {
                    return;
                }
                let at = spans.get(&id.get()).copied().unwrap_or(span);
                let leaving = std::mem::replace(expr, Expr::LitNull(NodeId::fresh()));
                let mut stmts = asserts(interner, rules, type_name, &base, at, false);
                stmts.push(Spanned::new(Stmt::Expr(leaving), at));
                *expr = Expr::Block(Block { stmts }, NodeId::fresh());
            },
            &mut |_, _| {},
        );
        before_returns(interner, checked, type_name, &base, false, body);
        before_returns(interner, kept, type_name, &base, true, body);
        let falls_through = !matches!(body.stmts.last().map(|s| &s.node), Some(Stmt::Return(_)));
        if !ret_type && falls_through {
            let at = body.stmts.last().map_or(span, |s| s.span);
            let end = Span::new(at.end as usize, at.end as usize);
            body.stmts
                .extend(asserts(interner, rules, type_name, &base, end, false));
        }
    }
    // **Where a value of the type is read, its rule holds** (D3): each
    // parameter of the type, and `self` in the type's methods.
    let mut given = Vec::new();
    let self_rules = self_rules.filter(|(o, _)| !changed.contains(*o));
    let entry = Span::new(span.start as usize, span.start as usize);
    if let (Some((type_name, rules)), Some(_)) = (self_rules, &receiver) {
        let base = Expr::Variable(self_name, NodeId::fresh());
        given.extend(asserts(interner, rules, type_name, &base, entry, true));
    }
    for arg in &args {
        let type_name = named(interner, &arg.ty);
        let Some(rules) = ruled.get(&type_name) else {
            continue;
        };
        if changed.contains(&type_name) || !arg.ty.generics.is_empty() || arg.ty.is_tuple {
            continue;
        }
        let base = Expr::Variable(arg.name, NodeId::fresh());
        given.extend(asserts(interner, rules, &type_name, &base, entry, true));
    }
    if !given.is_empty() {
        given.append(&mut body.stmts);
        body.stmts = given;
    }
}

/// **Every literal of a type with a rule is checked where it is built** (D2).
fn check_literals(
    interner: &InternerContext,
    ruled: &BTreeMap<String, Ruled>,
    spans: &BTreeMap<u32, Span>,
    body: &mut Block,
    span: Span,
) {
    crate::text_tiers::visit_block_mut(
        body,
        &mut |expr| {
            let Expr::StructLit { name, id, .. } = expr else {
                return;
            };
            let type_name = interner.resolve(*name).to_string();
            let Some(rules) = ruled.get(&type_name) else {
                return;
            };
            let at = spans.get(&id.get()).copied().unwrap_or(span);
            let literal = std::mem::replace(expr, Expr::LitNull(NodeId::fresh()));
            let binding = Expr::Variable(rules.binding, NodeId::fresh());
            // **Fields that are plain values are read as they are**: the rule
            // is asserted over them before the literal, so that the prover
            // sees `slots.len()` rather than a field of a new binding.
            if let Some(values) = plain_fields(&literal) {
                let mut checks = Block {
                    stmts: asserts(interner, rules, &type_name, &binding, at, false),
                };
                crate::text_tiers::visit_block_mut(
                    &mut checks,
                    &mut |e| {
                        if let Expr::Field { base, name, .. } = e
                            && matches!(&**base, Expr::Variable(b, _) if *b == rules.binding)
                            && let Some(value) = values.get(name)
                        {
                            *e = renumbered(value);
                        }
                    },
                    &mut |_, _| {},
                );
                let mut stmts = checks.stmts;
                stmts.push(Spanned::new(Stmt::Expr(literal), at));
                *expr = Expr::Block(Block { stmts }, NodeId::fresh());
                return;
            }
            let mut stmts = vec![Spanned::new(
                Stmt::Let {
                    names: vec![rules.binding],
                    shape: Vec::new(),
                    mutable: false,
                    ty: None,
                    value: literal,
                },
                at,
            )];
            stmts.extend(asserts(interner, rules, &type_name, &binding, at, false));
            stmts.push(Spanned::new(
                Stmt::Expr(Expr::Variable(rules.binding, NodeId::fresh())),
                at,
            ));
            *expr = Expr::Block(Block { stmts }, NodeId::fresh());
        },
        &mut |_, _| {},
    );
}

/// The literal's field values by name, where each is a name or a literal:
/// a value read twice means the same.
fn plain_fields(literal: &Expr) -> Option<BTreeMap<Symbol, Expr>> {
    let Expr::StructLit { fields, .. } = literal else {
        return None;
    };
    let mut out = BTreeMap::new();
    for field in fields {
        let value = match &field.value {
            None => Expr::Variable(field.name, NodeId::fresh()),
            Some(
                v @ (Expr::Variable(..)
                | Expr::LitInt { .. }
                | Expr::LitBool(..)
                | Expr::LitChar(..)
                | Expr::LitFloat(..)),
            ) => v.clone(),
            Some(_) => return None,
        };
        out.insert(field.name, value);
    }
    Some(out)
}

/// A `return` written as a statement, anywhere in the body but a lambda:
/// the rule is checked before it.
fn before_returns(
    interner: &InternerContext,
    rules: &Ruled,
    type_name: &str,
    base: &Expr,
    given: bool,
    block: &mut Block,
) {
    let stmts = std::mem::take(&mut block.stmts);
    for mut stmt in stmts {
        match &mut stmt.node {
            Stmt::Return(_) => {
                block.stmts.extend(asserts_as(
                    interner, rules, type_name, base, stmt.span, given, true,
                ));
            }
            Stmt::For { body, .. } | Stmt::While { body, .. } => {
                before_returns(interner, rules, type_name, base, given, body);
            }
            Stmt::Let { value, .. } | Stmt::Expr(value) => {
                nested_returns(interner, rules, type_name, base, given, value);
            }
            _ => {}
        }
        block.stmts.push(stmt);
    }
}

/// The blocks an expression holds as statements of its own: an `if`'s
/// branches, a `match`'s arms, a block.
fn nested_returns(
    interner: &InternerContext,
    rules: &Ruled,
    type_name: &str,
    base: &Expr,
    given: bool,
    expr: &mut Expr,
) {
    match expr {
        Expr::If {
            then_branch,
            else_branch,
            ..
        } => {
            before_returns(interner, rules, type_name, base, given, then_branch);
            if let Some(other) = else_branch {
                before_returns(interner, rules, type_name, base, given, other);
            }
        }
        Expr::Block(block, _) | Expr::Unsafe(block, _) => {
            before_returns(interner, rules, type_name, base, given, block);
        }
        Expr::Match { arms, .. } => {
            for arm in arms {
                nested_returns(interner, rules, type_name, base, given, &mut arm.body);
            }
        }
        _ => {}
    }
}

/// **A package of several files keeps a rule as a fact only where no other
/// file can break it** (D3). The rules are written out one file at a time
/// ([`expand`]), so here a literal of a type another file declares is checked
/// as one in the type's own file is (D2), its rules read again in the file
/// that builds it; a method of the type that changes `self` in another file,
/// and a field of it written, set by a `with` or handed on there, each as
/// [`changed_outside`] reads them, take the type's rule out of the facts at
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
        for item in &mut file.program.items {
            let span = item.span;
            match &mut item.node {
                Item::Impl { methods, .. } => {
                    for method in methods {
                        let span = method.span;
                        if let Item::Fn { body, .. } = &mut method.node {
                            check_literals(&interner, &ruled, &spans, body, span);
                        }
                    }
                }
                Item::Fn { body, .. } | Item::Test { body, .. } | Item::Bench { body, .. } => {
                    check_literals(&interner, &ruled, &spans, body, span);
                }
                _ => {}
            }
        }
    }
    if changed.is_empty() {
        return;
    }
    for (file, _) in files.iter_mut() {
        let interner = file.interner.clone();
        for item in &mut file.program.items {
            match &mut item.node {
                Item::Impl { methods, .. } => {
                    for method in methods {
                        drop_given(&interner, &changed, &mut method.node);
                    }
                }
                other => drop_given(&interner, &changed, other),
            }
        }
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

/// A function's entry without the facts of the rules of `changed`.
fn drop_given(interner: &InternerContext, changed: &BTreeSet<String>, item: &mut Item) {
    let body = match item {
        Item::Fn { body, .. } | Item::Test { body, .. } | Item::Bench { body, .. } => body,
        _ => return,
    };
    drop_given_in(interner, changed, body);
}

fn is_self(interner: &InternerContext, expr: &Expr) -> bool {
    matches!(expr, Expr::Variable(n, _) if interner.resolve(*n) == "self")
}

fn expr_id(expr: &Expr) -> u32 {
    crate::check::value_node(expr) as u32
}

/// **After a call of a method of the type that changes `self`, the rule
/// holds again** (D2, D3): the call's exits hold it, so each statement whose
/// value is such a call - `self.m(…)`, `let x = self.m(…)`, `x = self.m(…)` -
/// is followed by the rule, given.
fn given_after_calls(
    interner: &InternerContext,
    rules: &Ruled,
    type_name: &str,
    changing: &BTreeSet<String>,
    block: &mut Block,
) {
    let changes_self = |e: &Expr| {
        matches!(e, Expr::MethodCall { receiver, method, .. }
            if is_self(interner, receiver) && changing.contains(interner.resolve(*method)))
    };
    let stmts = std::mem::take(&mut block.stmts);
    for mut stmt in stmts {
        let called = match &mut stmt.node {
            Stmt::For { body, .. } | Stmt::While { body, .. } => {
                given_after_calls(interner, rules, type_name, changing, body);
                false
            }
            Stmt::Let { value, .. } | Stmt::Expr(value) => {
                nested_given(interner, rules, type_name, changing, value);
                changes_self(value)
            }
            Stmt::Assign { target, value, .. } => {
                matches!(target, Expr::Variable(..))
                    && !is_self(interner, target)
                    && changes_self(value)
            }
            _ => false,
        };
        let at = stmt.span;
        block.stmts.push(stmt);
        if called {
            let base = Expr::Variable(interner.intern_string("self"), NodeId::fresh());
            block
                .stmts
                .extend(asserts(interner, rules, type_name, &base, at, true));
        }
    }
}

/// The blocks an expression holds as statements of its own, for
/// [`given_after_calls`].
fn nested_given(
    interner: &InternerContext,
    rules: &Ruled,
    type_name: &str,
    changing: &BTreeSet<String>,
    expr: &mut Expr,
) {
    match expr {
        Expr::If {
            then_branch,
            else_branch,
            ..
        } => {
            given_after_calls(interner, rules, type_name, changing, then_branch);
            if let Some(other) = else_branch {
                given_after_calls(interner, rules, type_name, changing, other);
            }
        }
        Expr::Block(block, _) | Expr::Unsafe(block, _) => {
            given_after_calls(interner, rules, type_name, changing, block);
        }
        Expr::Match { arms, .. } => {
            for arm in arms {
                nested_given(interner, rules, type_name, changing, &mut arm.body);
            }
        }
        _ => {}
    }
}

/// The given rules of a type another file changes, in every block of a
/// body: one at an entry or after a call is dropped; one a method keeps
/// ([`kept_by`]), which also carries its `message:`, is a check again.
fn drop_given_in(interner: &InternerContext, changed: &BTreeSet<String>, body: &mut Block) {
    let drop = |stmts: &mut Vec<Spanned<Stmt>>| {
        stmts.retain_mut(|stmt| {
            let Stmt::Expr(Expr::Call { config, .. }) = &mut stmt.node else {
                return true;
            };
            let given = |arg: &ConfigArg| {
                interner.resolve(arg.name) == GIVEN
                    && matches!(&arg.value, Expr::LitStr { text, .. } if changed.contains(text))
            };
            if !config.iter().any(given) {
                return true;
            }
            if config
                .iter()
                .any(|arg| interner.resolve(arg.name) == "message")
            {
                config.retain(|arg| !given(arg));
                return true;
            }
            false
        });
    };
    drop(&mut body.stmts);
    crate::text_tiers::visit_block_mut(
        body,
        &mut |expr| match expr {
            Expr::Block(block, _) | Expr::Unsafe(block, _) | Expr::Closure { body: block, .. } => {
                drop(&mut block.stmts)
            }
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                drop(&mut then_branch.stmts);
                if let Some(other) = else_branch {
                    drop(&mut other.stmts);
                }
            }
            _ => {}
        },
        &mut |stmt, _| {
            if let Stmt::For { body, .. } | Stmt::While { body, .. } = stmt {
                drop(&mut body.stmts);
            }
        },
    );
}
