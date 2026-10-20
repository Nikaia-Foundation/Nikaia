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
        let mut small = type_name.clone();
        if let Some(first) = small.get(0..1) {
            small = format!("{}{}", first.to_lowercase(), &small[1..]);
        }
        if field_names.contains(&small) || small == type_name {
            small = format!("{small}_value");
        }
        ruled.insert(
            type_name,
            Ruled {
                fields: field_names,
                rules: read,
                binding: interner.intern_string(&small),
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
        } else {
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

fn rewrite_fn(
    interner: &InternerContext,
    ruled: &BTreeMap<String, Ruled>,
    changed: &BTreeSet<String>,
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
    // **Every literal of a type with a rule is checked where it is built**.
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
    let self_name = interner.intern_string("self");
    let self_rules = owner.and_then(|o| ruled.get(o).map(|r| (o, r)));
    // **A method that changes its receiver keeps the rule when it leaves**.
    if let (Some((type_name, rules)), Some(how)) = (self_rules, &receiver)
        && how.is_mut
    {
        let base = Expr::Variable(self_name, NodeId::fresh());
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
        before_returns(interner, rules, type_name, &base, body);
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

/// A `return` written as a statement, anywhere in the body but a lambda:
/// the rule is checked before it.
fn before_returns(
    interner: &InternerContext,
    rules: &Ruled,
    type_name: &str,
    base: &Expr,
    block: &mut Block,
) {
    let stmts = std::mem::take(&mut block.stmts);
    for mut stmt in stmts {
        match &mut stmt.node {
            Stmt::Return(_) => {
                block
                    .stmts
                    .extend(asserts(interner, rules, type_name, base, stmt.span, false));
            }
            Stmt::For { body, .. } | Stmt::While { body, .. } => {
                before_returns(interner, rules, type_name, base, body);
            }
            Stmt::Let { value, .. } | Stmt::Expr(value) => {
                nested_returns(interner, rules, type_name, base, value);
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
    expr: &mut Expr,
) {
    match expr {
        Expr::If {
            then_branch,
            else_branch,
            ..
        } => {
            before_returns(interner, rules, type_name, base, then_branch);
            if let Some(other) = else_branch {
                before_returns(interner, rules, type_name, base, other);
            }
        }
        Expr::Block(block, _) | Expr::Unsafe(block, _) => {
            before_returns(interner, rules, type_name, base, block);
        }
        Expr::Match { arms, .. } => {
            for arm in arms {
                nested_returns(interner, rules, type_name, base, &mut arm.body);
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
/// ([`expand`]), so a file does not check a rule of a type another file
/// declares: a literal of it, a method of it that changes `self`, and a
/// field of it written, set by a `with` or handed on, each as
/// [`changed_outside`] reads them, take the type's rule out of the facts at
/// every entry of the package.
pub(crate) fn across_files(files: &mut [&mut Parsed]) {
    if files.len() < 2 {
        return;
    }
    let declared: Vec<BTreeMap<String, BTreeSet<String>>> =
        files.iter().map(|file| ruled_fields(file)).collect();
    if declared.iter().all(BTreeMap::is_empty) {
        return;
    }
    let mut changed = BTreeSet::new();
    for (at, file) in files.iter_mut().enumerate() {
        let foreign: BTreeMap<String, BTreeSet<String>> = declared
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != at)
            .flat_map(|(_, types)| types.clone())
            .collect();
        if foreign.is_empty() {
            continue;
        }
        let interner = file.interner.clone();
        changed.extend(changed_outside(
            &interner,
            &foreign,
            &mut file.program.items,
        ));
        changed.extend(built_elsewhere(
            &interner,
            &foreign,
            &mut file.program.items,
        ));
    }
    if changed.is_empty() {
        return;
    }
    for file in files.iter_mut() {
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

/// The structs of a file that state rules, with their fields' names.
fn ruled_fields(parsed: &Parsed) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    for item in &parsed.program.items {
        if let Item::Struct {
            name,
            fields,
            rules,
            ..
        } = &item.node
            && !rules.is_empty()
        {
            out.insert(
                parsed.interner.resolve(*name).to_string(),
                fields
                    .iter()
                    .map(|f| parsed.interner.resolve(f.name).to_string())
                    .collect(),
            );
        }
    }
    out
}

/// The types of `ruled` this file builds a literal of, or writes a method
/// for that changes `self`: neither is checked here.
fn built_elsewhere(
    interner: &InternerContext,
    ruled: &BTreeMap<String, BTreeSet<String>>,
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
                        if ruled.contains_key(&type_name) {
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
    body.stmts.retain(|stmt| {
        let Stmt::Expr(Expr::Call { config, .. }) = &stmt.node else {
            return true;
        };
        !config.iter().any(|arg| {
            interner.resolve(arg.name) == GIVEN
                && matches!(&arg.value, Expr::LitStr { text, .. } if changed.contains(text))
        })
    });
}

fn is_self(interner: &InternerContext, expr: &Expr) -> bool {
    matches!(expr, Expr::Variable(n, _) if interner.resolve(*n) == "self")
}

fn expr_id(expr: &Expr) -> u32 {
    crate::check::value_node(expr) as u32
}
