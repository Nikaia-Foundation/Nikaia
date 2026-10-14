// crates/nikaia/src/view_changes.rs
//
// **What happens to data while a view of it is still used**
// ([ADR-353](../../docs/specification/adr/adr-353.md) D3, D4; Part I 6.6).
//
// A view is a local bound to a call that hands back views of its arguments
// (`let r = longest(x, y)`, the ledger's `borrows`) or to an element or a
// field read off a place (`let e = ys[0]`, ADR-094 D4, ADR-354). While such a
// view is still used - a later statement of the block that binds it names it,
// or a loop around the change does - the data it may point into is watched:
//
// * a **change in place** of that data, a method that changes the value or an
//   assignment to a part of it, is `NK2305`, at the change (D4);
// * **giving it up** by assigning the variable a new value is not a change: the
//   old buffer lives on (D3). Where the assignment stands in the block that
//   declares the variable, the lowering binds the new value as a name of its
//   own (`let y = v`), and the old one lives to the end of that block, longer
//   than any view of it there - no copy. The end of a scope is the tether's
//   own case (`contracts::keep`): the buffer is put into the frame's keep.
//
// The walk reads the tree and both ledgers, as `views` does; whether a `let`
// lends a place is the checker's fact (`lent_lets`), handed in.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::ast::{Block, Expr, Item, Span, Stmt};
use crate::check::Finding;
use crate::contracts::Ledger;
use crate::contracts::sync::{visit_stmt, visit_stmt_blocks};
use crate::parser::Parsed;

/// What the lowering does at a give-up: an assignment written as a binding of
/// its own (by the statement's first byte, with whether that binding is
/// changed later), and the `let`s that no longer need `mut` because every
/// change to them became such a binding.
#[derive(Default, Debug, Clone)]
pub struct GiveUps {
    pub rebound: HashMap<i64, bool>,
    pub unmuted: HashSet<i64>,
}

#[derive(Clone)]
struct View {
    name: String,
    roots: BTreeSet<String>,
    /// The statement that made it, and the call or the place it came from.
    made: Span,
    from: String,
    /// The callee's way out that may hand back the root, where it is known.
    way_out: HashMap<String, String>,
    /// The first later statement that uses it, where one does.
    used: Option<Span>,
}

struct Walk<'a> {
    parsed: &'a Parsed,
    own: &'a Ledger,
    library: &'a Ledger,
    lent: &'a BTreeSet<i64>,
    findings: Vec<Finding>,
    give_ups: GiveUps,
}

/// The refusals (`NK2305`) and the give-ups of one unit.
pub fn analyse(
    parsed: &Parsed,
    own: &Ledger,
    library: &Ledger,
    lent: &BTreeSet<i64>,
) -> (Vec<Finding>, GiveUps) {
    let mut walk = Walk {
        parsed,
        own,
        library,
        lent,
        findings: Vec::new(),
        give_ups: GiveUps::default(),
    };
    for item in &parsed.program.items {
        match &item.node {
            Item::Fn { body, .. } => walk.function(body),
            Item::Impl { methods, .. } => {
                for method in methods {
                    if let Item::Fn { body, .. } = &method.node {
                        walk.function(body);
                    }
                }
            }
            _ => {}
        }
    }
    (walk.findings, walk.give_ups)
}

fn root_of(parsed: &Parsed, expr: &Expr) -> Option<String> {
    match expr {
        Expr::Variable(name, _) => Some(parsed.text(*name).to_string()),
        Expr::Field { base, .. } | Expr::Index { base, .. } | Expr::SafeField { base, .. } => {
            root_of(parsed, base)
        }
        Expr::Unary { expr, .. } => root_of(parsed, expr),
        _ => None,
    }
}

fn mentions_stmt(parsed: &Parsed, stmt: &Stmt, name: &str) -> bool {
    let mut found = false;
    visit_stmt(parsed, stmt, &mut |each| {
        if let Expr::Variable(n, _) = each
            && parsed.text(*n) == name
        {
            found = true;
        }
    });
    if !found {
        visit_stmt_blocks(stmt, &mut |inner| {
            if !found && mentions_block(parsed, inner, name) {
                found = true;
            }
        });
    }
    found
}

fn mentions_block(parsed: &Parsed, block: &Block, name: &str) -> bool {
    block
        .stmts
        .iter()
        .any(|s| mentions_stmt(parsed, &s.node, name))
}

impl Walk<'_> {
    fn function(&mut self, body: &Block) {
        self.block(body, &[]);
    }

    fn contract(&self, key: &str) -> Option<&crate::contracts::FnContract> {
        self.own
            .functions
            .get(key)
            .or_else(|| self.library.functions.get(key))
    }

    /// Whether every method of that name the ledgers know changes its subject.
    fn changing_method(&self, method: &str) -> bool {
        let suffix = format!("::{method}");
        let mut count = 0;
        for ledger in [self.own, self.library] {
            for (key, contract) in &ledger.functions {
                if key.ends_with(&suffix) {
                    count += 1;
                    if !contract.mutates {
                        return false;
                    }
                }
            }
        }
        count > 0
    }

    /// The view a `let` binds: the roots it may point into, where it is one.
    fn view_of(&self, at: &Span, name: &str, value: &Expr, outer: &[View]) -> Option<View> {
        let parsed = self.parsed;
        match value {
            Expr::Variable(other, _) => {
                let other = parsed.text(*other);
                let held = outer.iter().rev().find(|v| v.name == other)?;
                Some(View {
                    name: name.to_string(),
                    made: *at,
                    ..held.clone()
                })
            }
            Expr::Index { .. } | Expr::Field { .. } if self.lent.contains(&(at.start as i64)) => {
                let root = root_of(parsed, value)?;
                Some(View {
                    name: name.to_string(),
                    roots: BTreeSet::from([root.clone()]),
                    made: *at,
                    from: format!("`{name}` is a view into `{root}`"),
                    way_out: HashMap::new(),
                    used: None,
                })
            }
            Expr::Call { func, args, .. } => {
                let Expr::Variable(callee, _) = &**func else {
                    return None;
                };
                let callee = parsed.text(*callee).to_string();
                let contract = self.contract(&callee)?;
                let signature = contract.signature.as_ref()?;
                let mut roots = BTreeSet::new();
                let mut way_out = HashMap::new();
                for borrowed in &contract.borrows {
                    let Some(index) = signature.params.iter().position(|(p, _)| p == borrowed)
                    else {
                        continue;
                    };
                    let Some(root) = args.get(index).and_then(|a| root_of(parsed, a)) else {
                        continue;
                    };
                    way_out.insert(root.clone(), self.returning(&callee, borrowed));
                    roots.insert(root);
                }
                if roots.is_empty() {
                    return None;
                }
                Some(View {
                    name: name.to_string(),
                    roots,
                    made: *at,
                    from: format!("`{name}` is made here by `{callee}`"),
                    way_out,
                    used: None,
                })
            }
            _ => None,
        }
    }

    /// The way out of `callee` that hands back `param`: `return b`, where the
    /// body is in this unit.
    fn returning(&self, callee: &str, param: &str) -> String {
        let parsed = self.parsed;
        for item in &parsed.program.items {
            let Item::Fn {
                name: Some(name),
                body,
                ..
            } = &item.node
            else {
                continue;
            };
            if parsed.text(*name) != callee {
                continue;
            }
            let mut found = false;
            fn each(parsed: &Parsed, block: &Block, param: &str, found: &mut bool) {
                for stmt in &block.stmts {
                    if let Stmt::Return(Some(value)) = &stmt.node
                        && root_of(parsed, value).as_deref() == Some(param)
                    {
                        *found = true;
                    }
                    visit_stmt_blocks(&stmt.node, &mut |inner| each(parsed, inner, param, found));
                }
            }
            each(parsed, body, param, &mut found);
            if found {
                return format!("`{callee}` may hand back `{param}`: `return {param}`.");
            }
        }
        format!("`{callee}` may hand back what it was given as `{param}`.")
    }

    fn block(&mut self, block: &Block, outer: &[View]) {
        let parsed = self.parsed;
        let mut views: Vec<View> = outer.to_vec();
        let own_from = views.len();
        // The `let`s of this block, by name: the statement and `mut`.
        let mut declared: HashMap<String, (i64, bool)> = HashMap::new();
        for (j, stmt) in block.stmts.iter().enumerate() {
            let later = &block.stmts[j + 1..];
            for view in views.iter_mut().skip(own_from) {
                view.used = later
                    .iter()
                    .find(|s| mentions_stmt(parsed, &s.node, &view.name))
                    .map(|s| s.span);
            }
            let looping = matches!(stmt.node, Stmt::For { .. } | Stmt::While { .. });
            let at = stmt.span;
            self.changes_in(&stmt.node, &at, &views);
            match &stmt.node {
                Stmt::Assign { target, op, .. } => {
                    let whole = op.is_none() && matches!(target, Expr::Variable(..));
                    if let Some(root) = root_of(parsed, target) {
                        let viewed = views
                            .iter()
                            .any(|v| v.used.is_some() && v.roots.contains(&root));
                        if !whole && viewed {
                            self.refuse(&root, &at, &views);
                        } else if whole && viewed && declared.contains_key(&root) {
                            // Given up (D3): the old data lives on under the
                            // old binding, the new value is a binding of its own.
                            let changed = later.iter().any(|s| self.changes(&s.node, &root));
                            self.give_ups.rebound.insert(at.start as i64, changed);
                        }
                        if whole {
                            // The name holds other data now.
                            for view in views.iter_mut() {
                                view.roots.remove(&root);
                            }
                        }
                    }
                }
                Stmt::Let {
                    names,
                    value,
                    mutable,
                    ..
                } if names.len() == 1 => {
                    let name = parsed.text(names[0]).to_string();
                    views.retain(|v| v.name != name);
                    for view in views.iter_mut() {
                        view.roots.remove(&name);
                    }
                    if let Some(view) = self.view_of(&at, &name, value, &views) {
                        views.push(view);
                    }
                    declared.insert(name, (at.start as i64, *mutable));
                }
                _ => {}
            }
            // Nested blocks: a loop around a change uses what its body names.
            let mut inner_views = views.clone();
            if looping {
                for view in inner_views.iter_mut() {
                    if view.used.is_none() && mentions_stmt(parsed, &stmt.node, &view.name) {
                        view.used = Some(at);
                    }
                }
            }
            let mut nested: Vec<&Block> = Vec::new();
            visit_stmt_blocks(&stmt.node, &mut |inner| nested.push(inner));
            for inner in nested {
                self.block(inner, &inner_views);
            }
        }
        // A `let mut` whose every change became a binding of its own.
        for (name, (at, mutable)) in declared {
            if !mutable {
                continue;
            }
            let mut rebound = false;
            let mut other = false;
            for stmt in &block.stmts {
                let start = stmt.span.start as i64;
                if start == at {
                    continue;
                }
                if self.give_ups.rebound.contains_key(&start)
                    && matches!(&stmt.node, Stmt::Assign { target: Expr::Variable(n, _), .. } if parsed.text(*n) == name)
                {
                    rebound = true;
                } else if self.changes(&stmt.node, &name) {
                    other = true;
                }
            }
            if rebound && !other {
                self.give_ups.unmuted.insert(at);
            }
        }
    }

    /// Whether a statement changes `name`, or may: an assignment to it or a
    /// part of it, a method that changes it, or a call that is handed it.
    fn changes(&self, stmt: &Stmt, name: &str) -> bool {
        let parsed = self.parsed;
        let mut found = false;
        if let Stmt::Assign { target, .. } = stmt
            && root_of(parsed, target).as_deref() == Some(name)
        {
            return true;
        }
        visit_stmt(parsed, stmt, &mut |e| match e {
            Expr::MethodCall {
                receiver, method, ..
            } if root_of(parsed, receiver).as_deref() == Some(name)
                && !self.reads_only(parsed.text(*method)) =>
            {
                found = true
            }
            Expr::Call { func, args, .. } => {
                for (index, arg) in args.iter().enumerate() {
                    if root_of(parsed, arg).as_deref() == Some(name)
                        && self.changes_argument(func, index)
                    {
                        found = true
                    }
                }
            }
            _ => {}
        });
        let mut nested: Vec<&Block> = Vec::new();
        visit_stmt_blocks(stmt, &mut |inner| nested.push(inner));
        found
            || nested
                .iter()
                .any(|b| b.stmts.iter().any(|s| self.changes(&s.node, name)))
    }

    /// Whether a call may change its argument `index`: the callee declares
    /// that parameter `mut`, or the ledgers do not know the callee.
    fn changes_argument(&self, func: &Expr, index: usize) -> bool {
        let Expr::Variable(callee, _) = func else {
            return true;
        };
        let Some(contract) = self.contract(self.parsed.text(*callee)) else {
            return true;
        };
        let Some(signature) = contract.signature.as_ref() else {
            return true;
        };
        signature
            .params
            .get(index)
            .is_none_or(|(param, _)| signature.mutable.contains(param))
    }

    /// Whether every method of that name the ledgers know leaves its subject.
    fn reads_only(&self, method: &str) -> bool {
        let suffix = format!("::{method}");
        let mut count = 0;
        for ledger in [self.own, self.library] {
            for (key, contract) in &ledger.functions {
                if key.ends_with(&suffix) {
                    count += 1;
                    if contract.mutates {
                        return false;
                    }
                }
            }
        }
        count > 0
    }

    /// The changes in place among a statement's own expressions.
    fn changes_in(&mut self, stmt: &Stmt, at: &Span, views: &[View]) {
        let parsed = self.parsed;
        let mut changed: Vec<String> = Vec::new();
        visit_stmt(parsed, stmt, &mut |e| {
            if let Expr::MethodCall {
                receiver, method, ..
            } = e
                && let Some(root) = root_of(parsed, receiver)
                && self.changing_method(parsed.text(*method))
            {
                changed.push(root);
            }
        });
        for root in changed {
            if views
                .iter()
                .any(|v| v.used.is_some() && v.roots.contains(&root))
            {
                self.refuse(&root, at, views);
            }
        }
    }

    fn refuse(&mut self, root: &str, at: &Span, views: &[View]) {
        let Some(view) = views
            .iter()
            .rev()
            .find(|v| v.used.is_some() && v.roots.contains(root))
        else {
            return;
        };
        let label = |span: Span, word: &str, text: String, main: bool| {
            nikaia_std::tools::check_state::CheckLabel {
                span,
                word: word.to_string(),
                text,
                main,
            }
        };
        let name = &view.name;
        let mut labels = vec![
            label(*at, root, format!("`{root}` changes here"), true),
            label(view.made, name, view.from.clone(), false),
        ];
        if let Some(used) = view.used {
            labels.push(label(
                used,
                name,
                format!("`{name}` is used here, after the change"),
                false,
            ));
        }
        let mut notes = Vec::new();
        if let Some(way) = view.way_out.get(root) {
            notes.push(way.clone());
        }
        self.findings.push(Finding {
            severity: crate::check::Severity::Error,
            span: *at,
            code: "NK2305".to_string(),
            message: format!("`{root}` is changed here, but `{name}` may point into its data and is used later."),
            notes,
            help: Some(format!(
                "Use `{name}` before this change, or take a copy of it with `{name}.clone()` before the change."
            )),
            labels,
        });
    }
}
