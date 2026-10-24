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

/// The option that marks a rule's `assert` as given rather than claimed: a
/// fact at a function's entry (ADR-369 D3). A name with spaces, which no
/// program can write.
pub const GIVEN: &str = "rule of the type";

/// A struct of the file that states rules.
struct Ruled {
    fields: BTreeSet<String>,
    /// Each rule's claim and its `message:`, as written.
    rules: Vec<(Expr, Option<Expr>, Span)>,
    /// The name a checked literal is bound to: the type's name, its first
    /// letter small (`piece`), which is what a failure shows.
    binding: Symbol,
}

/// Writes every rule of the file's structs out as its `assert`s; `NK1288`
/// for a rule that is not one.
pub(crate) fn expand(parsed: &mut Parsed) -> anyhow::Result<()> {
    let interner = parsed.interner.clone();
    let mut ruled: BTreeMap<String, Ruled> = BTreeMap::new();
    for item in &parsed.program.items {
        let Item::Struct {
            name,
            fields,
            rules,
            generics,
            ..
        } = &item.node
        else {
            continue;
        };
        if rules.is_empty() {
            continue;
        }
        let type_name = interner.resolve(*name).to_string();
        let field_names: BTreeSet<String> = fields
            .iter()
            .map(|f| interner.resolve(f.name).to_string())
            .collect();
        let mut read = Vec::new();
        for rule in rules {
            let Some((claim, message)) = a_rule(&interner, &rule.node) else {
                return Err(refused(
                    rule.span,
                    format!("A rule of `{type_name}` has to be an `assert`."),
                    "Write it as `assert(claim)` or `assert(claim; message: \"…\")` after the fields.",
                ));
            };
            if !generics.is_empty() {
                return Err(refused(
                    rule.span,
                    format!("`{type_name}` is generic, and a generic struct states no rule yet."),
                    "Check what the methods need with an `assert` in each of them.",
                ));
            }
            let probe = Expr::Variable(*name, NodeId::fresh());
            if on(&interner, claim, &probe, &field_names).is_none() {
                return Err(refused(
                    rule.span,
                    format!(
                        "A rule of `{type_name}` may read only its fields, paths and pure methods from them, `comptime` values and literals."
                    ),
                    "Compute anything else in a method, and state the rule over the fields it keeps.",
                ));
            }
            read.push((claim.clone(), message.cloned(), rule.span));
        }
        let binding = binding_of(&interner, &type_name, &field_names);
        ruled.insert(
            type_name,
            Ruled {
                fields: field_names,
                rules: read,
                binding,
            },
        );
    }
    if ruled.is_empty() {
        return Ok(());
    }
    let spans = parsed.expr_spans.clone();
    let mut items = std::mem::take(&mut parsed.program.items);
    let fields: BTreeMap<String, BTreeSet<String>> = ruled
        .iter()
        .map(|(name, r)| (name.clone(), r.fields.clone()))
        .collect();
    let mut changed = changed_outside(&interner, &fields, &mut items);
    changed.extend(fails_changed(&interner, &ruled, &items));
    // Each type's methods in this file, and of them those that change `self`.
    let mut own: BTreeMap<String, Methods> = BTreeMap::new();
    for item in &items {
        let Item::Impl {
            target, methods, ..
        } = &item.node
        else {
            continue;
        };
        let entry = own
            .entry(interner.resolve(target.name).to_string())
            .or_default();
        for method in methods {
            if let Item::Fn {
                name: Some(name),
                receiver: Some(how),
                ..
            } = &method.node
            {
                let name = interner.resolve(*name).to_string();
                if how.is_mut {
                    entry.changing.insert(name.clone());
                }
                entry.all.insert(name);
            }
        }
    }
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

/// The name a checked literal of `type_name` is bound to: its first letter
/// small (`piece`), which is what a failure shows.
fn binding_of(interner: &InternerContext, type_name: &str, fields: &BTreeSet<String>) -> Symbol {
    let mut small = type_name.to_string();
    if let Some(first) = small.get(0..1) {
        small = format!("{}{}", first.to_lowercase(), &small[1..]);
    }
    if fields.contains(&small) || small == type_name {
        small = format!("{small}_value");
    }
    interner.intern_string(&small)
}

/// **The types a method can leave broken through a failure it passes on**:
/// a method that changes its receiver and `throws` leaves at every call that
/// fails, and a caller that catches the failure still holds the value, its
/// rule maybe broken for a while. The rule is checked before each `return`
/// and `throw` written, not before each call that may fail, so the rule of
/// such a type is not a fact at an entry (D3): what is lost is a proof, never
/// a check.
fn fails_changed(
    interner: &InternerContext,
    ruled: &BTreeMap<String, Ruled>,
    items: &[Spanned<Item>],
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for item in items {
        let Item::Impl {
            target, methods, ..
        } = &item.node
        else {
            continue;
        };
        let owner = interner.resolve(target.name).to_string();
        if !ruled.contains_key(&owner) {
            continue;
        }
        let fails = methods.iter().any(|m| {
            matches!(
                &m.node,
                Item::Fn {
                    receiver: Some(how),
                    can_throw: true,
                    ..
                } if how.is_mut
            )
        });
        if fails {
            out.insert(owner);
        }
    }
    out
}

/// The claim and the `message:` of `assert(claim; message: …)`.
fn a_rule<'a>(interner: &InternerContext, rule: &'a Expr) -> Option<(&'a Expr, Option<&'a Expr>)> {
    let Expr::Call {
        func, args, config, ..
    } = rule
    else {
        return None;
    };
    let Expr::Variable(called, _) = &**func else {
        return None;
    };
    if interner.resolve(*called) != "assert" || args.len() != 1 {
        return None;
    }
    let mut message = None;
    for option in config {
        if interner.resolve(option.name) != "message" {
            return None;
        }
        message = Some(&option.value);
    }
    Some((&args[0], message))
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

/// The claim with each field read through `base`, every node new; `None`
/// where it reads something a rule may not.
fn on(
    interner: &InternerContext,
    claim: &Expr,
    base: &Expr,
    fields: &BTreeSet<String>,
) -> Option<Expr> {
    let fresh = NodeId::fresh;
    let again = |e: &Expr| on(interner, e, base, fields);
    Some(match claim {
        Expr::Variable(name, _) => {
            if fields.contains(interner.resolve(*name)) {
                Expr::Field {
                    base: Box::new(renumbered(base)),
                    name: *name,
                    id: fresh(),
                }
            } else {
                Expr::Variable(*name, fresh())
            }
        }
        Expr::Path(segments, _) => Expr::Path(segments.clone(), fresh()),
        Expr::LitInt {
            value, negative, ..
        } => Expr::LitInt {
            value: *value,
            negative: *negative,
            id: fresh(),
        },
        Expr::LitBool(b, _) => Expr::LitBool(*b, fresh()),
        Expr::LitChar(c, _) => Expr::LitChar(c.clone(), fresh()),
        Expr::LitFloat(f, _) => Expr::LitFloat(f.clone(), fresh()),
        Expr::LitNull(_) => Expr::LitNull(fresh()),
        Expr::Binary {
            op,
            lhs,
            rhs,
            span,
            grouped,
            ..
        } => Expr::Binary {
            op: *op,
            lhs: Box::new(again(lhs)?),
            rhs: Box::new(again(rhs)?),
            span: *span,
            grouped: *grouped,
            id: fresh(),
        },
        Expr::Unary { op, expr, .. } => Expr::Unary {
            op: *op,
            expr: Box::new(again(expr)?),
            id: fresh(),
        },
        Expr::Field {
            base: inner, name, ..
        } => Expr::Field {
            base: Box::new(again(inner)?),
            name: *name,
            id: fresh(),
        },
        Expr::MethodCall {
            receiver,
            method,
            args,
            config,
            ..
        } if config.is_empty() => Expr::MethodCall {
            receiver: Box::new(again(receiver)?),
            method: *method,
            args: args.iter().map(again).collect::<Option<Vec<_>>>()?,
            config: Vec::new(),
            id: fresh(),
        },
        Expr::Cast { expr, ty, .. } => Expr::Cast {
            expr: Box::new(again(expr)?),
            ty: ty.clone(),
            id: fresh(),
        },
        _ => return None,
    })
}

/// A base read once more: the same name or path, a node of its own.
fn renumbered(base: &Expr) -> Expr {
    match base {
        Expr::Variable(name, _) => Expr::Variable(*name, NodeId::fresh()),
        Expr::Field { base, name, .. } => Expr::Field {
            base: Box::new(renumbered(base)),
            name: *name,
            id: NodeId::fresh(),
        },
        other => {
            let mut copy = other.clone();
            crate::ast::set_id(&mut copy, NodeId::fresh());
            copy
        }
    }
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
    asserts_as(interner, ruled, type_name, base, span, given, !given)
}

/// [`asserts`], given where `given` says so and with its `message:` where
/// `said` does: a rule a method keeps ([`kept_by`]) is both, so that it is a
/// check again where another file breaks the rule ([`drop_given_in`]).
fn asserts_as(
    interner: &InternerContext,
    ruled: &Ruled,
    type_name: &str,
    base: &Expr,
    span: Span,
    given: bool,
    said: bool,
) -> Vec<Spanned<Stmt>> {
    let mut out = Vec::new();
    for (claim, message, _) in &ruled.rules {
        let Some(condition) = on(interner, claim, base, &ruled.fields) else {
            continue;
        };
        let mut config = Vec::new();
        if given {
            config.push(ConfigArg {
                name: interner.intern_string(GIVEN),
                value: Expr::LitStr {
                    text: type_name.to_string(),
                    id: NodeId::fresh(),
                },
            });
        }
        if said {
            // **A failure names the type** (D2): its message is the rule's,
            // after the type's name.
            let said = match message {
                None => Expr::LitStr {
                    text: format!("the rule of `{type_name}`"),
                    id: NodeId::fresh(),
                },
                Some(Expr::LitStr { text, .. }) => Expr::LitStr {
                    text: format!("the rule of `{type_name}`: {text}"),
                    id: NodeId::fresh(),
                },
                Some(said) => {
                    let mut said = said.clone();
                    crate::ast::set_id(&mut said, NodeId::fresh());
                    said
                }
            };
            config.push(ConfigArg {
                name: interner.intern_string("message"),
                value: said,
            });
        }
        let call = Expr::Call {
            func: Box::new(Expr::Variable(
                interner.intern_string("assert"),
                NodeId::fresh(),
            )),
            args: vec![condition],
            config,
            id: NodeId::fresh(),
        };
        out.push(Spanned::new(Stmt::Expr(call), span));
    }
    out
}

/// The name a type is written by: `Piece`, `ref Piece`, `mut Piece`.
fn named(interner: &InternerContext, ty: &Type) -> String {
    interner.resolve(ty.name).to_string()
}

/// A type's methods in the file: every one that takes `self`, and those that
/// change it.
#[derive(Default)]
struct Methods {
    all: BTreeSet<String>,
    changing: BTreeSet<String>,
}

#[allow(clippy::too_many_arguments)]
fn rewrite_fn(
    interner: &InternerContext,
    ruled: &BTreeMap<String, Ruled>,
    changed: &BTreeSet<String>,
    own: &BTreeMap<String, Methods>,
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
    let no_methods = Methods::default();
    let methods = owner.and_then(|o| own.get(o)).unwrap_or(&no_methods);
    // **A rule the method cannot break is not checked in it** ([`kept_by`]):
    // given at each point instead, with its message for where another file
    // breaks it.
    let mut kept: Option<Ruled> = None;
    let mut checked: Option<Ruled> = None;
    if let (Some((type_name, rules)), Some(how)) = (self_rules, &receiver)
        && how.is_mut
    {
        let mut keeps = Vec::new();
        let mut checks = Vec::new();
        for rule in &rules.rules {
            if !changed.contains(type_name)
                && kept_by(interner, &rule.0, &rules.fields, &methods.all, body)
            {
                keeps.push(rule.clone());
            } else {
                checks.push(rule.clone());
            }
        }
        let part = |rules_of: Vec<(Expr, Option<Expr>, Span)>| Ruled {
            fields: rules.fields.clone(),
            rules: rules_of,
            binding: rules.binding,
        };
        kept = Some(part(keeps));
        checked = Some(part(checks));
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

/// The methods that only read the field they are called on: a call of one
/// on a field from outside the type's methods leaves its rule as it was.
const READERS: [&str; 12] = [
    "len",
    "is_empty",
    "get",
    "contains",
    "contains_key",
    "iter",
    "clone",
    "first",
    "last",
    "keys",
    "values",
    "text",
];

/// **The types whose fields this file may change outside their own
/// methods**, where a rule is not checked after the change: a field of
/// one written, set by a `with`, handed to a call or called a method on that
/// may change it, anywhere but through `self` in the type's methods. Read
/// by the field's name, whatever its owner, so a type it names is left out
/// even where the change is another type's: the rule is then not a fact at
/// an entry (D3), and what is lost is a proof, never a check.
fn changed_outside(
    interner: &InternerContext,
    ruled: &BTreeMap<String, BTreeSet<String>>,
    items: &mut [Spanned<Item>],
) -> BTreeSet<String> {
    let owners_of = |field: Symbol| -> Vec<String> {
        let text = interner.resolve(field);
        ruled
            .iter()
            .filter(|(_, fields)| fields.contains(text))
            .map(|(name, _)| name.clone())
            .collect()
    };
    let is_self = |e: &Expr| matches!(e, Expr::Variable(n, _) if interner.resolve(*n) == "self");
    let changed = std::cell::RefCell::new(BTreeSet::new());
    for item in items.iter_mut() {
        let owner = match &item.node {
            Item::Impl { target, .. } => Some(interner.resolve(target.name).to_string()),
            _ => None,
        };
        crate::text_tiers::each_block(&mut item.node, &mut |block| {
            crate::text_tiers::visit_block_mut(
                block,
                &mut |expr| match expr {
                    Expr::With { fields, .. } => {
                        for field in fields.iter() {
                            changed.borrow_mut().extend(owners_of(field.name));
                        }
                    }
                    Expr::MethodCall {
                        receiver,
                        method,
                        args,
                        ..
                    } => {
                        if let Expr::Field { base, name, .. } = &**receiver
                            && !(is_self(base) && owner.is_some())
                            && !READERS.contains(&interner.resolve(*method))
                        {
                            changed.borrow_mut().extend(owners_of(*name));
                        }
                        for arg in args.iter() {
                            if let Expr::Field { base, name, .. } = arg
                                && !(is_self(base) && owner.is_some())
                            {
                                changed.borrow_mut().extend(owners_of(*name));
                            }
                        }
                    }
                    Expr::Call { args, .. } => {
                        for arg in args.iter() {
                            if let Expr::Field { base, name, .. } = arg
                                && !(is_self(base) && owner.is_some())
                            {
                                changed.borrow_mut().extend(owners_of(*name));
                            }
                        }
                    }
                    _ => {}
                },
                &mut |stmt, _| {
                    let Stmt::Assign { target, .. } = stmt else {
                        return;
                    };
                    let mut place = &*target;
                    loop {
                        match place {
                            Expr::Field { base, name, .. } => {
                                if !(is_self(base) && owner.is_some()) {
                                    changed.borrow_mut().extend(owners_of(*name));
                                }
                                place = base;
                            }
                            Expr::Index { base, .. } => place = base,
                            _ => break,
                        }
                    }
                },
            );
        });
    }
    changed.into_inner()
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
    let declared: Vec<BTreeMap<String, Declared>> = files
        .iter()
        .map(|(file, source)| declared_rules(file, source))
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
        let foreign: BTreeMap<String, &Declared> = declared
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
        changed.extend(changed_outside(&interner, &fields, &mut file.program.items));
        changed.extend(built_elsewhere(
            &interner,
            &fields,
            &ruled,
            &mut file.program.items,
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

/// A struct of a file that states rules, as another file reads it.
struct Declared {
    fields: BTreeSet<String>,
    /// Each rule as written, `assert(…)`, to be read again in the file that
    /// builds a literal of the type.
    rules: Vec<String>,
    /// A rule reads a `pub` field, or the type is `pub` and every field is
    /// `pub` or has a default: another package can break the rule unchecked.
    open: bool,
}

/// The structs of a file that state rules.
fn declared_rules(parsed: &Parsed, source: &str) -> BTreeMap<String, Declared> {
    let interner = &parsed.interner;
    let mut out = BTreeMap::new();
    for item in &parsed.program.items {
        let Item::Struct {
            name,
            fields,
            rules,
            is_public,
            ..
        } = &item.node
        else {
            continue;
        };
        if rules.is_empty() {
            continue;
        }
        let public: BTreeSet<String> = fields
            .iter()
            .filter(|f| f.is_public)
            .map(|f| interner.resolve(f.name).to_string())
            .collect();
        let buildable = *is_public && fields.iter().all(|f| f.is_public || f.default.is_some());
        let reads_public = rules.iter().any(|rule| {
            a_rule(interner, &rule.node).is_none_or(|(claim, _)| reads(interner, claim, &public))
        });
        out.insert(
            interner.resolve(*name).to_string(),
            Declared {
                fields: fields
                    .iter()
                    .map(|f| interner.resolve(f.name).to_string())
                    .collect(),
                rules: rules
                    .iter()
                    .map(|rule| {
                        source
                            .get(rule.span.start as usize..rule.span.end as usize)
                            .unwrap_or_default()
                            .to_string()
                    })
                    .collect(),
                open: buildable || reads_public,
            },
        );
    }
    out
}

/// Whether `claim` reads one of `names` as a field.
fn reads(interner: &InternerContext, claim: &Expr, names: &BTreeSet<String>) -> bool {
    let again = |e: &Expr| reads(interner, e, names);
    match claim {
        Expr::Variable(name, _) => names.contains(interner.resolve(*name)),
        Expr::Binary { lhs, rhs, .. } => again(lhs) || again(rhs),
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } => again(expr),
        Expr::Field { base, .. } => again(base),
        Expr::MethodCall { receiver, args, .. } => again(receiver) || args.iter().any(again),
        Expr::Path(..)
        | Expr::LitInt { .. }
        | Expr::LitBool(..)
        | Expr::LitChar(..)
        | Expr::LitFloat(..)
        | Expr::LitNull(_) => false,
        _ => true,
    }
}

/// A type's rules read again in `file`, with its words: `None` where one
/// does not read as a rule, and a literal of the type is then not checked.
fn read_again(file: &Parsed, type_name: &str, declared: &Declared) -> Option<Ruled> {
    let interner = &file.interner;
    let mut rules = Vec::new();
    for text in &declared.rules {
        let rule = file.read_expression(text).ok()?;
        let (claim, message) = a_rule(interner, &rule)?;
        rules.push((claim.clone(), message.cloned(), Span::new(0, 0)));
    }
    Some(Ruled {
        binding: binding_of(interner, type_name, &declared.fields),
        fields: declared.fields.clone(),
        rules,
    })
}

/// The types of `ruled` this file writes a method for that changes `self`,
/// or builds a literal of without reading its rules again (`checked`):
/// neither is checked here.
fn built_elsewhere(
    interner: &InternerContext,
    ruled: &BTreeMap<String, BTreeSet<String>>,
    checked: &BTreeMap<String, Ruled>,
    items: &mut [Spanned<Item>],
) -> BTreeSet<String> {
    let found = std::cell::RefCell::new(BTreeSet::new());
    for item in items.iter_mut() {
        if let Item::Impl {
            target, methods, ..
        } = &item.node
        {
            let owner = interner.resolve(target.name).to_string();
            let changes_self = methods
                .iter()
                .any(|m| matches!(&m.node, Item::Fn { receiver: Some(how), .. } if how.is_mut));
            if changes_self && ruled.contains_key(&owner) {
                found.borrow_mut().insert(owner);
            }
        }
        crate::text_tiers::each_block(&mut item.node, &mut |block| {
            crate::text_tiers::visit_block_mut(
                block,
                &mut |expr| {
                    if let Expr::StructLit { name, .. } = expr {
                        let type_name = interner.resolve(*name).to_string();
                        if ruled.contains_key(&type_name) && !checked.contains_key(&type_name) {
                            found.borrow_mut().insert(type_name);
                        }
                    }
                },
                &mut |_, _| {},
            );
        });
    }
    found.into_inner()
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

/// **What a rule reads of each field**: `true` where only its length
/// (`xs.len()`), `false` where anything more.
fn rule_reads(
    interner: &InternerContext,
    claim: &Expr,
    fields: &BTreeSet<String>,
    out: &mut BTreeMap<String, bool>,
) {
    match claim {
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            if let Expr::Variable(name, _) = &**receiver
                && fields.contains(interner.resolve(*name))
                && interner.resolve(*method) == "len"
                && args.is_empty()
            {
                out.entry(interner.resolve(*name).to_string())
                    .or_insert(true);
                return;
            }
            rule_reads(interner, receiver, fields, out);
            for arg in args {
                rule_reads(interner, arg, fields, out);
            }
        }
        Expr::Variable(name, _) if fields.contains(interner.resolve(*name)) => {
            out.insert(interner.resolve(*name).to_string(), false);
        }
        Expr::Binary { lhs, rhs, .. } => {
            rule_reads(interner, lhs, fields, out);
            rule_reads(interner, rhs, fields, out);
        }
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } => {
            rule_reads(interner, expr, fields, out);
        }
        Expr::Field { base, .. } => rule_reads(interner, base, fields, out),
        _ => {}
    }
}

/// **A rule a method cannot break** (D2, proved by what the method does):
/// every use of `self` in its body reads a field the rule does not read,
/// counts one it reads only the length of (`self.xs.len()`) or indexes it
/// (`self.xs[i]`, which keeps its length), or calls a method of the type,
/// whose exits hold the rule. The rule then holds wherever the body stands as
/// it held at the entry, and no point of it needs a check.
fn kept_by(
    interner: &InternerContext,
    claim: &Expr,
    fields: &BTreeSet<String>,
    methods: &BTreeSet<String>,
    body: &mut Block,
) -> bool {
    let mut reads = BTreeMap::new();
    rule_reads(interner, claim, fields, &mut reads);
    let field_of_self = |e: &Expr| match e {
        Expr::Field { base, name, .. } if is_self(interner, base) => {
            Some(interner.resolve(*name).to_string())
        }
        _ => None,
    };
    let mut seen = 0usize;
    let mut allowed = 0usize;
    crate::text_tiers::visit_block_mut(
        body,
        &mut |expr| match &*expr {
            Expr::Variable(..) if is_self(interner, expr) => seen += 1,
            Expr::Field { .. } => {
                if let Some(name) = field_of_self(expr)
                    && !reads.contains_key(&name)
                {
                    allowed += 1;
                }
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                let method = interner.resolve(*method);
                if is_self(interner, receiver) && methods.contains(method) {
                    allowed += 1;
                } else if let Some(name) = field_of_self(receiver)
                    && reads.contains_key(&name)
                    && method == "len"
                    && args.is_empty()
                {
                    allowed += 1;
                }
            }
            Expr::Index { base, .. } => {
                if let Some(name) = field_of_self(base)
                    && reads.get(&name) == Some(&true)
                {
                    allowed += 1;
                }
            }
            _ => {}
        },
        &mut |_, _| {},
    );
    seen == allowed
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
