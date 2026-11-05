//! **What a declared `String` is below, decided by what flows into it**
//! ([ADR-282](../../../docs/specification/adr/adr-282.md),
//! [ADR-282](../../../docs/specification/adr/adr-282.md)).
//!
//! [ADR-282](../../../docs/specification/adr/adr-282.md) D1 made `String` the
//! one text type and its state - borrowed, tethered, owned - the compiler's,
//! *the cheapest state that works, chosen per use*. Below, a declaration has
//! one representation, so for a declared `String` "per use" means: by every
//! value any line of the package puts there. A **position** is every place a
//! program declares `String`: a struct's field, a function's result, a
//! function's parameter, an annotated `let` - and inside each, the element of
//! a list or a set and the key or value of a map (`Vec[String]`,
//! `HashMap[String, i64]`). Three tiers, and each pays only for itself:
//!
//! * **only text of its own** flows in - the ordinary case: it stays `String`,
//!   and nothing about it changes;
//! * **only views** (and literals, which are views of text that lives as long
//!   as the program): it becomes `ref String`, exactly as if the program had
//!   written that, and [ADR-283](../../../docs/specification/adr/adr-283.md)
//!   decides where each buffer lives - in the caller's frame for nothing, with
//!   a handle only where no frame outlives it;
//! * **both**: it becomes `ref String` marked [`crate::ast::Type::either`],
//!   which is `nikaia_std::either_text::EitherText` below. A view is borrowed
//!   and text of its own is owned, **each at its own line**, so no line pays for
//!   another line's kind of text.
//!
//! A position **only literals** (and views of them) flow into is a view of the
//! program's own static text, `&'static str` below: it borrows no buffer, so it
//! carries no lifetime and nothing is copied
//! ([ADR-282](../../../docs/specification/adr/adr-282.md) D29). A position
//! literals and text of its own flow into stays `String`, and the literal is
//! built where it stands as before (D4).
//!
//! **What is published may become a view but never both kinds**: a `pub` field
//! of a `pub` struct, and a `pub` function's parameters and result. Their
//! representation leaves with the package, before the programs that use it
//! exist; a view the package itself puts there is in its ledger as
//! `ref String`, and a mixed one would need a type no other package can know
//! to build. There the view is refused as before, saying why
//! ([ADR-282](../../../docs/specification/adr/adr-282.md) D25).
//!
//! Run once, right after parsing, so that the checker, every derived column,
//! the ledger and the emitter all read one answer off the types.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Block, Expr, Item, Stmt};
use crate::parser::Parsed;

/// **No two nodes of the tree share an id** ([ADR-340](../../../docs/specification/adr/adr-340.md)
/// D2): a node the parser placed twice - `xs[a..]` holds `xs` as the base and
/// again as the receiver of `len` - keeps its id where it is first met, and
/// the second place takes a new one, so what is recorded about one place is not
/// read at the other.
fn separate_repeated_ids(parsed: &mut Parsed) {
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    for item in &mut parsed.program.items {
        each_block(&mut item.node, &mut |block| {
            visit_block_mut(
                block,
                &mut |expr| {
                    if !seen.insert(crate::ast::id_of(expr)) {
                        crate::ast::set_id(expr, crate::ast::NodeId::fresh());
                    }
                },
                &mut |_, _| {},
            )
        });
    }
}

/// **Decide the tiers and write them into the types**, so that every later
/// reader of the program reads one answer.
///
/// **The walk and the decision are Nikaia** (`tools/text_tiers.nika`,
/// ADR-294, #125): every value's kind of text, the positions it flows into,
/// the fixpoint over them, where a value is handed into a mixed one, and
/// writing that answer into the types (`tier_write`). What stays here is
/// wrapping a value handed into a mixed position, which replaces a node.
pub fn refine(parsed: &mut Parsed) {
    refine_with(parsed, &BTreeMap::new());
}

/// The published fields of the published structs of a package's files, as
/// written (`tier_beside_fields`).
pub type BesideFields = BTreeMap<String, Vec<nikaia_std::tools::ty::FieldContract>>;

/// **One package's files, each decided against the published fields of all
/// of them** ([ADR-282](../../../docs/specification/adr/adr-282.md) D17, D25,
/// #956): a name bound to a literal and kept in a `String` of a struct
/// another file declares is declared `String`. The fields are read before
/// any file is decided, so the answer does not hang on the order.
pub fn refine_package(files: &mut [&mut Parsed]) {
    let read: Vec<&Parsed> = files.iter().map(|parsed| &**parsed).collect();
    let beside = beside_fields(&read);
    for parsed in files.iter_mut() {
        refine_with(parsed, &beside);
    }
}

/// What `refine_package` reads of the files (`tools/text_tiers.nika`).
pub fn beside_fields(files: &[&Parsed]) -> BesideFields {
    let views: Vec<_> = files
        .iter()
        .map(|parsed| crate::check::parsed_view_of(parsed))
        .collect();
    nikaia_std::tools::text_tiers::tier_beside_fields(&views)
}

/// One file, against the published fields of its package's files.
pub fn refine_with(parsed: &mut Parsed, beside: &BesideFields) {
    separate_repeated_ids(parsed);
    use nikaia_std::tools::text_tiers::{TierAsk, hand_number, text_tiers, tier_write};
    let library = crate::contracts::std_ledger();
    let methods = nikaia_std::tools::ty::method_index(library);
    let ask = TierAsk {
        names: &parsed.interner,
        aliases: &parsed.aliases,
        library,
        methods: &methods,
        beside,
    };
    let decided = text_tiers(&parsed.program, &ask);
    parsed.viewed_arms = decided.viewed_arms.clone();
    parsed.refused_moves = decided.refused_moves.clone();
    // A parameter handed back is a view of it, of any type (ADR-views-3 D12, #1239).
    nikaia_std::tools::text_tiers::tier_hand_back(&mut parsed.program, &parsed.interner);
    if decided.tiers.is_empty() && decided.lets.is_empty() && decided.adopted.is_empty() {
        return;
    }
    let wraps: BTreeMap<usize, usize> = decided
        .wraps
        .iter()
        .map(|(at, hand)| (*at as usize, hand_number(*hand) as usize))
        .collect();
    let into = [
        parsed.interner.intern_string("into_either"),
        parsed.interner.intern_string("into_either_maybe"),
        parsed.interner.intern_string("either_items"),
        parsed.interner.intern_string("either_keys"),
        parsed.interner.intern_string("either_values"),
        parsed.interner.intern_string("either_pairs"),
    ];
    parsed.hole_wraps = decided
        .hole_wraps
        .iter()
        .map(|(at, wraps)| {
            // By the value's id, which is what the hole is matched by wherever
            // a reader takes a copy of it (ADR-340).
            let nodes: BTreeSet<(usize, usize)> = wraps
                .iter()
                .map(|wrap| {
                    (
                        crate::check::value_node(&wrap.value) as usize,
                        hand_number(wrap.hand) as usize,
                    )
                })
                .collect();
            let wraps = nodes
                .into_iter()
                .map(|(node, hand)| (node, into[hand]))
                .collect();
            (*at as u32, wraps)
        })
        .collect();
    // **The `String` a binding is declared with does not have to be written
    // anywhere else** (#441): `text` is the type node of a declared field,
    // parameter or result, and a program with none of them - a `main` that
    // pushes `let n = "ada"` into a `Vec[String]` - had no node, so D17's
    // binding stayed a view and the push was refused.
    let text = decided.text.clone().or_else(|| {
        (!decided.lets.is_empty()).then(|| crate::ast::Type {
            name: parsed.interner.intern_string("String"),
            generics: Vec::new(),
            is_view: false,
            is_tuple: false,
            is_nullable: false,
            code: Box::new(None),
            count: None,
            is_mut: false,
            is_slice: false,
            either: false,
            lasting: false,
        })
    });
    // Values first: a value is found by the id the walk saw it by.
    for item in &mut parsed.program.items {
        wrap_item(&mut item.node, &wraps, into);
    }
    let written = tier_write(&mut parsed.program, &parsed.interner, &decided, text);
    parsed.text_tiers = written.said;
    parsed.text_views = written.views;
}

// --- Walking the program mutably ------------------------------------------

pub(crate) fn each_block(item: &mut Item, f: &mut dyn FnMut(&mut Block)) {
    match item {
        Item::Fn { body, .. } | Item::Test { body, .. } | Item::Bench { body, .. } => f(body),
        Item::Impl { methods, .. } => {
            for method in methods {
                each_block(&mut method.node, f);
            }
        }
        _ => {}
    }
}

/// Every expression and block below a block, children before parents.
pub(crate) fn visit_block_mut(
    block: &mut Block,
    f: &mut dyn FnMut(&mut Expr),
    s: &mut dyn FnMut(&mut Stmt, usize),
) {
    for stmt in &mut block.stmts {
        let at = stmt.span.at();
        match &mut stmt.node {
            Stmt::Let { value, .. } => visit_expr_mut(value, f, s),
            Stmt::Assign { target, value, .. } => {
                visit_expr_mut(target, f, s);
                visit_expr_mut(value, f, s);
            }
            Stmt::For { iter, body, .. } => {
                visit_expr_mut(iter, f, s);
                visit_block_mut(body, f, s);
            }
            Stmt::While { cond, body } => {
                visit_expr_mut(cond, f, s);
                visit_block_mut(body, f, s);
            }
            Stmt::Return(Some(value)) | Stmt::Expr(value) => visit_expr_mut(value, f, s),
            _ => {}
        }
        s(&mut stmt.node, at);
    }
}

pub(crate) fn visit_expr_mut(
    expr: &mut Expr,
    f: &mut dyn FnMut(&mut Expr),
    s: &mut dyn FnMut(&mut Stmt, usize),
) {
    match expr {
        Expr::Call {
            func, args, config, ..
        } => {
            visit_expr_mut(func, f, s);
            args.iter_mut().for_each(|a| visit_expr_mut(a, f, s));
            config
                .iter_mut()
                .for_each(|c| visit_expr_mut(&mut c.value, f, s));
        }
        Expr::MethodCall {
            receiver,
            args,
            config,
            ..
        }
        | Expr::SafeMethod {
            receiver,
            args,
            config,
            ..
        } => {
            visit_expr_mut(receiver, f, s);
            args.iter_mut().for_each(|a| visit_expr_mut(a, f, s));
            config
                .iter_mut()
                .for_each(|c| visit_expr_mut(&mut c.value, f, s));
        }
        Expr::Binary { lhs, rhs, .. } => {
            visit_expr_mut(lhs, f, s);
            visit_expr_mut(rhs, f, s);
        }
        Expr::Unary { expr, .. }
        | Expr::Try(expr, _)
        | Expr::Async(expr, _, _)
        | Expr::Throw(expr, _)
        | Expr::Cast { expr, .. } => visit_expr_mut(expr, f, s),
        Expr::Return(value, _) => {
            if let Some(value) = &mut **value {
                visit_expr_mut(value, f, s)
            }
        }
        Expr::Field { base, .. } | Expr::SafeField { base, .. } => visit_expr_mut(base, f, s),
        Expr::Index { base, index, .. } => {
            visit_expr_mut(base, f, s);
            visit_expr_mut(index, f, s);
        }
        Expr::Range { start, end, .. } => {
            visit_expr_mut(start, f, s);
            visit_expr_mut(end, f, s);
        }
        Expr::Tuple(items, _) | Expr::ListLit { items, .. } => {
            items.iter_mut().for_each(|i| visit_expr_mut(i, f, s))
        }
        Expr::ListRepeat { value, count, .. } => {
            visit_expr_mut(value, f, s);
            visit_expr_mut(count, f, s);
        }
        Expr::Coalesce {
            value, fallback, ..
        } => {
            visit_expr_mut(value, f, s);
            visit_expr_mut(fallback, f, s);
        }
        Expr::TryCatch { expr, handler, .. } => {
            visit_expr_mut(expr, f, s);
            visit_block_mut(handler, f, s);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            visit_expr_mut(cond, f, s);
            visit_block_mut(then_branch, f, s);
            if let Some(block) = else_branch {
                visit_block_mut(block, f, s);
            }
        }
        Expr::Match { value, arms, .. } => {
            visit_expr_mut(value, f, s);
            for arm in arms {
                if let Some(guard) = &mut arm.guard {
                    visit_expr_mut(guard, f, s);
                }
                visit_expr_mut(&mut arm.body, f, s);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                if let Some(value) = &mut field.value {
                    visit_expr_mut(value, f, s);
                }
            }
        }
        Expr::With { base, fields, .. } => {
            visit_expr_mut(base, f, s);
            for field in fields {
                if let Some(value) = &mut field.value {
                    visit_expr_mut(value, f, s);
                }
            }
        }
        Expr::Block(block, _)
        | Expr::Unsafe(block, _)
        | Expr::Overlap(block, _)
        | Expr::AsyncGroup(block, _, _) => visit_block_mut(block, f, s),
        Expr::Closure { body, .. } => visit_block_mut(body, f, s),
        Expr::Spawn { body, .. } => visit_expr_mut(body, f, s),
        _ => {}
    }
    f(expr);
}

/// **A value going into a mixed position is handed over as `value.into_either()`**
/// (ADR-282 D14): `EitherText` borrows a view and moves text of its own in,
/// and which it is the language below reads off the value's type.
fn wrap_item(item: &mut Item, wraps: &BTreeMap<usize, usize>, into: [winnow_grammar::Symbol; 6]) {
    if wraps.is_empty() {
        return;
    }
    // Found by the id the parser gave it (ADR-340 D4), and replaced where the
    // walk meets it: children are visited before their parent, so wrapping one
    // moves nothing another comparison is still waiting for, and the wrapper
    // takes an id of its own.
    each_block(item, &mut |block| {
        visit_block_mut(
            block,
            &mut |expr| {
                if let Some(hand) = wraps.get(&(crate::check::value_node(expr) as usize)) {
                    wrapped(expr, into[*hand]);
                }
            },
            &mut |_, _| {},
        )
    });
}

/// `expr` becomes `expr.<method>()`: the value moves into the receiver's box
/// and the call is a node of its own.
fn wrapped(expr: &mut Expr, method: winnow_grammar::Symbol) {
    let value = std::mem::replace(expr, Expr::LitNull(crate::ast::NodeId::fresh()));
    *expr = Expr::MethodCall {
        receiver: Box::new(value),
        method,
        args: Vec::new(),
        config: Vec::new(),
        id: crate::ast::NodeId::fresh(),
    };
}

/// **Apply a hole's wraps to it** (ADR-229 D1): each value whose id was
/// recorded becomes `value.<method>()`, as [`wrap_item`] does to the program.
/// The wrapper takes an id of its own and what it wraps keeps its own, so a
/// reader of the copy finds what the checker recorded about the value.
pub fn wrap_hole(hole: &mut Expr, wraps: &[(usize, winnow_grammar::Symbol)]) {
    visit_expr_mut(
        hole,
        &mut |expr| {
            let id = crate::check::value_node(expr);
            if let Some((_, method)) = wraps.iter().find(|(wanted, _)| *wanted == (id as usize)) {
                wrapped(expr, *method);
            }
        },
        &mut |_, _| {},
    );
}
