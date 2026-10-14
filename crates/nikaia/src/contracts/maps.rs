//! **The ledger key `maps`** ([ADR-010](../../../../docs/specification/adr/adr-010.md)
//! D9, Part III 13.5, #581 step 6): for a `pub` function with a map in its
//! signature, which of those map positions are one map, and what its body
//! puts into each - `from(p, ...)` (keys as trusted as the arguments named,
//! `from()` for none) or `keyed`.
//!
//! ## A permission key, so every doubt is `keyed`
//!
//! A caller reads the line and never the body. A wrong `from(..)` lets a caller
//! give the fixed seed to a map the body fills with keys an adversary chose, so
//! the body is read in a few shapes only and everything else is `keyed`:
//!
//!   * a map is read (`len`, `get`, `keys`, ...), filled by `m[k] = v` or
//!     `m.insert(k, ..)`, returned, or put in the struct the function returns;
//!   * a key is a literal, a parameter, or built from those with a method that
//!     only reshapes it (`clone`, `trim`, ...); a `let` or a `for` binding
//!     carries what it is made of.
//!
//! Anything else - a map handed to a call, kept in a `let`, compared with
//! something unseen, a key from a call or from a name nothing explains, a
//! `match`, a closure - makes **every** map of the function `keyed`, and its
//! positions are then grouped by the type written (the safe grouping for the
//! types, [`Shape::exact`]).
//!
//! ## Positions, groups, hidden parameters
//!
//! A position is one place a map type is written in the signature: a
//! parameter, a map field of a `pub` struct a parameter has (`t.counts`), or
//! the result likewise. A `pub` type declares one hidden parameter per
//! position. A `pub` function declares one per **group of positions its body
//! joins** (a map assigned to another, compared, returned from a parameter):
//! the `|` in a line. Where the body is not read exactly, the positions of one
//! written type are one group.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{BinaryOp, Block, Expr, FPart, Item, Stmt, Type};
use crate::emit::seeds::Seeds;
use crate::parser::Parsed;

use super::Ledger;

const RESULT: &str = "<result>";
const SELF: &str = "self";

/// Methods that read a map and hand back something that is not a map.
const READS: &[&str] = &[
    "len",
    "is_empty",
    "contains_key",
    "contains",
    "capacity",
    "get",
    "get_or",
    "remove",
    "clear",
    "iter",
    "keys",
    "values",
];

/// Methods that put a key into a map.
const ADDS: &[&str] = &[
    "insert",
    "entry",
    "extend",
    "push",
    "add",
    "append",
    "get_or_insert",
];

/// Methods that make a key from the keys of their receiver and arguments and
/// nothing else.
const SHAPES: &[&str] = &[
    "clone",
    "len",
    "to_string",
    "trim",
    "trim_start",
    "trim_end",
    "to_lowercase",
    "to_uppercase",
    "as_str",
    "abs",
    "min",
    "max",
    "pow",
];

fn is_map(name: &str) -> bool {
    matches!(
        name.rsplit("::").next().unwrap_or(name),
        "HashMap" | "HashSet" | "TrustedMap" | "TrustedSet"
    )
}

/// One map position of a signature.
#[derive(Debug, Clone)]
struct Position {
    /// The name it has in a line: `m`, `t.counts`, `<result>.counts`.
    label: String,
    /// The parameter it belongs to (`self`, a name, or `<result>`).
    owner: String,
    /// The way down from the parameter: `""`, `".counts"`.
    path: String,
    /// The written type it is in, for the grouping that needs no body.
    group: String,
    /// A position of the type of the `impl` a method is in: the ordinal among
    /// that type's own parameters.
    within: Option<usize>,
}

/// A struct: the file it is in and its fields.
type Fields<'a> = (&'a Parsed, Vec<(String, &'a Type)>);

/// The structs of the units, by name.
struct Structs<'a> {
    by_name: BTreeMap<String, Fields<'a>>,
}

impl<'a> Structs<'a> {
    fn read(units: &[&'a Parsed]) -> Structs<'a> {
        let mut by_name = BTreeMap::new();
        for parsed in units {
            for item in &parsed.program.items {
                if let Item::Struct { name, fields, .. } = &item.node {
                    by_name.insert(
                        parsed.text(*name).to_string(),
                        (
                            *parsed,
                            fields
                                .iter()
                                .map(|f| (parsed.text(f.name).to_string(), &f.ty))
                                .collect(),
                        ),
                    );
                }
            }
        }
        Structs { by_name }
    }
}

struct Gather<'a> {
    seeds: &'a Seeds,
    structs: &'a Structs<'a>,
    /// The type of the `impl` the method is in.
    target: Option<&'a str>,
    /// A map holds another one: the positions are not independent lines.
    nested: bool,
}

/// Where a walk down a type is: the name it prints, the parameter, the struct
/// mention a field position belongs to, the way down.
struct At<'s> {
    owner: &'s str,
    label: &'s str,
    scope: &'s str,
    path: &'s str,
    /// Inside a mention of the `impl`'s own type: the ordinal of the next
    /// position among that type's parameters.
    within: Option<&'s std::cell::Cell<usize>>,
}

impl Gather<'_> {
    fn of(&mut self, parsed: &Parsed, ty: &Type, at: &At, out: &mut Vec<Position>, depth: usize) {
        if ty.code.is_some() || depth > 6 {
            return;
        }
        let name = parsed.text(ty.name);
        let ordinal = |at: &At| {
            at.within.map(|c| {
                let n = c.get();
                c.set(n + 1);
                n
            })
        };
        if !ty.is_tuple && is_map(name) {
            let key = self.seeds.key(parsed, ty, true);
            out.push(Position {
                label: at.label.to_string(),
                owner: at.owner.to_string(),
                path: at.path.to_string(),
                group: format!("{}|{key}|{}", at.scope, at.path),
                within: ordinal(at),
            });
            let before = out.len();
            for g in &ty.generics {
                self.of(parsed, g, at, out, depth + 1);
            }
            if out.len() > before {
                self.nested = true;
            }
            return;
        }
        let short = name.rsplit("::").next().unwrap_or(name);
        if !ty.is_tuple
            && self.seeds.arity(name) > 0
            && let Some((home, fields)) = self.structs.by_name.get(short)
        {
            let scope_here = if at.scope.is_empty() {
                self.seeds.key(parsed, ty, true)
            } else {
                at.scope.to_string()
            };
            // A mention of the `impl`'s own type starts counting from the
            // type's first parameter.
            let own = std::cell::Cell::new(0);
            let enters = self.target == Some(short) && at.within.is_none();
            for (field, fty) in fields {
                let label = format!("{}.{field}", at.label);
                let path = format!("{}.{field}", at.path);
                let inner = At {
                    owner: at.owner,
                    label: &label,
                    scope: &scope_here,
                    path: &path,
                    within: if enters { Some(&own) } else { at.within },
                };
                self.of(home, fty, &inner, out, depth + 1);
            }
        }
        for g in &ty.generics {
            self.of(parsed, g, at, out, depth + 1);
        }
    }
}

/// What a key is made of: names, or nothing a caller can vouch for.
#[derive(Debug, Clone, Default)]
struct Dep {
    unknown: bool,
    names: BTreeSet<String>,
}

impl Dep {
    fn unknown() -> Dep {
        Dep {
            unknown: true,
            names: BTreeSet::new(),
        }
    }

    fn join(mut self, other: Dep) -> Dep {
        self.unknown |= other.unknown;
        self.names.extend(other.names);
        self
    }
}

/// What a place written in the body is.
enum Res {
    /// Exactly one map position.
    Map(usize),
    /// A struct or a collection that holds some: not a map itself.
    Part,
    /// Neither: any other value.
    Not,
}

struct Body<'a> {
    parsed: &'a Parsed,
    /// The parameters, by name.
    params: BTreeSet<String>,
    /// The maps a name holds: the way down to each, and its root. A parameter
    /// has one per position, a local made with `HashMap()` one.
    roots: BTreeMap<String, Vec<(String, usize)>>,
    /// The name each root has in a key's dependencies (a parameter's), if any.
    root_name: Vec<Option<String>>,
    parent: Vec<usize>,
    adds: Vec<Dep>,
    /// Everything keyed: a shape this does not read.
    all_keyed: bool,
    /// What each local name is made of, joined over every place it is set.
    locals: BTreeMap<String, Dep>,
    /// The roots of the result, with their way down.
    result: Vec<(String, usize)>,
}

impl<'a> Body<'a> {
    fn find(&mut self, mut at: usize) -> usize {
        while self.parent[at] != at {
            self.parent[at] = self.parent[self.parent[at]];
            at = self.parent[at];
        }
        at
    }

    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.parent[a] = b;
        }
    }

    fn new_root(&mut self, name: Option<String>) -> usize {
        let id = self.parent.len();
        self.parent.push(id);
        self.adds.push(Dep::default());
        self.root_name.push(name);
        id
    }

    fn doubt(&mut self) {
        self.all_keyed = true;
    }

    fn add(&mut self, root: usize, dep: Dep) {
        let joined = std::mem::take(&mut self.adds[root]).join(dep);
        self.adds[root] = joined;
    }

    fn sym(&self, s: winnow_grammar::Symbol) -> String {
        self.parsed.text(s).to_string()
    }

    /// The variable and the way down of a chain of fields (an index on the way
    /// is the same position: a position is per type, not per element).
    fn locate(&mut self, e: &Expr) -> Option<(String, String)> {
        match e {
            Expr::Variable(n, _) => Some((self.sym(*n), String::new())),
            Expr::Field { base, name, .. } => {
                let (var, path) = self.locate(base)?;
                Some((var, format!("{path}.{}", self.parsed.text(*name))))
            }
            Expr::Index { base, index, .. } => {
                let found = self.locate(base)?;
                self.expr(index);
                Some(found)
            }
            _ => None,
        }
    }

    /// What a place is, without the keys an index adds.
    fn classify(&mut self, e: &Expr) -> Res {
        let Some((var, path)) = self.locate(e) else {
            return Res::Not;
        };
        let Some(held) = self.roots.get(&var) else {
            return Res::Not;
        };
        if let Some((_, id)) = held.iter().find(|(p, _)| *p == path) {
            return Res::Map(*id);
        }
        let below = format!("{path}.");
        if path.is_empty() || held.iter().any(|(p, _)| p.starts_with(&below)) {
            Res::Part
        } else {
            Res::Not
        }
    }

    /// The map a place names, with the keys it reads on the way down.
    fn place(&mut self, e: &Expr) -> Option<usize> {
        match e {
            Expr::Index { base, index, .. } => {
                let root = self.place(base)?;
                let dep = self.deps(index);
                self.add(root, dep);
                self.expr(index);
                Some(root)
            }
            Expr::Variable(..) | Expr::Field { .. } => match self.classify(e) {
                Res::Map(id) => Some(id),
                _ => None,
            },
            _ => None,
        }
    }

    /// The roots a value-place stands for, in position order.
    fn roots_of(&mut self, e: &Expr) -> Option<Vec<usize>> {
        let (var, path) = self.locate(e)?;
        let held = self.roots.get(&var)?;
        let below = format!("{path}.");
        let found: Vec<usize> = held
            .iter()
            .filter(|(p, _)| *p == path || p.starts_with(&below))
            .map(|(_, id)| *id)
            .collect();
        (!found.is_empty()).then_some(found)
    }

    fn map_made(&self, e: &Expr) -> bool {
        match e {
            Expr::Call { func, args, .. } if args.is_empty() => match &**func {
                Expr::Path(segments, _) => segments
                    .last()
                    .is_some_and(|s| matches!(self.parsed.text(*s), "HashMap" | "HashSet")),
                Expr::Variable(one, _) => matches!(self.parsed.text(*one), "HashMap" | "HashSet"),
                _ => false,
            },
            _ => false,
        }
    }

    fn deps(&self, e: &Expr) -> Dep {
        match e {
            Expr::LitInt { .. }
            | Expr::LitStr { .. }
            | Expr::LitChar(..)
            | Expr::LitBool(..)
            | Expr::LitFloat(..)
            | Expr::LitNull(_) => Dep::default(),
            Expr::Variable(n, _) => {
                let name = self.sym(*n);
                if self.params.contains(&name) || self.locals.contains_key(&name) {
                    Dep {
                        unknown: false,
                        names: BTreeSet::from([name]),
                    }
                } else {
                    Dep::unknown()
                }
            }
            Expr::Field { base, .. } | Expr::SafeField { base, .. } => self.deps(base),
            Expr::Index { base, index, .. } => self.deps(base).join(self.deps(index)),
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } if SHAPES.contains(&self.parsed.text(*method)) => args
                .iter()
                .fold(self.deps(receiver), |all, a| all.join(self.deps(a))),
            Expr::Binary { lhs, rhs, .. }
            | Expr::Coalesce {
                value: lhs,
                fallback: rhs,
                ..
            } => self.deps(lhs).join(self.deps(rhs)),
            Expr::Unary { expr, .. } | Expr::Cast { expr, .. } => self.deps(expr),
            Expr::Tuple(items, _) | Expr::ListLit { items, .. } => items
                .iter()
                .fold(Dep::default(), |all, a| all.join(self.deps(a))),
            Expr::LitInterpolated { parts, .. } => {
                parts.iter().fold(Dep::default(), |all, p| match p {
                    FPart::Hole { expr, .. } => all.join(self.deps(expr)),
                    _ => all,
                })
            }
            _ => Dep::unknown(),
        }
    }

    fn set_local(&mut self, name: String, dep: Dep) {
        if self.params.contains(&name) || self.roots.contains_key(&name) {
            // A name that hides a parameter: not followed.
            self.doubt();
            return;
        }
        let joined = self.locals.remove(&name).unwrap_or_default().join(dep);
        self.locals.insert(name, joined);
    }

    fn block(&mut self, block: &Block, tail_is_the_result: bool) {
        let last = block.stmts.len().saturating_sub(1);
        for (at, stmt) in block.stmts.iter().enumerate() {
            match &stmt.node {
                Stmt::Let { names, value, .. } => {
                    if names.len() == 1 && self.map_made(value) {
                        let name = self.sym(names[0]);
                        if self.params.contains(&name) || self.roots.contains_key(&name) {
                            self.doubt();
                        } else {
                            let root = self.new_root(None);
                            self.roots.insert(name, vec![(String::new(), root)]);
                        }
                        continue;
                    }
                    let dep = self.deps(value);
                    self.expr(value);
                    for n in names {
                        self.set_local(self.sym(*n), dep.clone());
                    }
                }
                Stmt::Assign { target, value, .. } => match target {
                    Expr::Variable(n, _) if !self.roots.contains_key(self.parsed.text(*n)) => {
                        let dep = self.deps(value);
                        self.expr(value);
                        self.set_local(self.sym(*n), dep);
                    }
                    Expr::Index { .. } => {
                        if self.place(target).is_none() {
                            self.expr(target);
                        }
                        self.expr(value);
                    }
                    // A whole map assigned, or a field that holds one.
                    _ => {
                        if !matches!(self.classify(target), Res::Not) && !self.map_made(value) {
                            self.doubt();
                        }
                        self.expr(value);
                    }
                },
                Stmt::For {
                    bindings,
                    iter,
                    body,
                    ..
                } => {
                    let dep = self.iterated(iter);
                    for b in bindings {
                        self.set_local(self.sym(*b), dep.clone());
                    }
                    self.block(body, false);
                }
                Stmt::While { cond, body } => {
                    self.expr(cond);
                    self.block(body, false);
                }
                Stmt::Return(value) => {
                    if let Some(v) = value {
                        self.returned(v);
                    }
                }
                Stmt::Expr(e) => {
                    if tail_is_the_result && at == last {
                        self.returned(e);
                    } else {
                        self.expr(e);
                    }
                }
                Stmt::Break | Stmt::Continue => {}
                Stmt::Comptime { .. } => self.doubt(),
            }
        }
    }

    /// What a `for` binds: the keys of a map it walks, or what the iterable
    /// is made of.
    fn iterated(&mut self, iter: &Expr) -> Dep {
        let walked = match iter {
            Expr::MethodCall {
                receiver, method, ..
            } if matches!(self.parsed.text(*method), "keys" | "values" | "iter") => {
                self.place(receiver)
            }
            other => self.place(other),
        };
        if let Some(root) = walked {
            return match self.root_name[root].clone() {
                Some(name) => Dep {
                    unknown: false,
                    names: BTreeSet::from([name]),
                },
                None => Dep::unknown(),
            };
        }
        let dep = self.deps(iter);
        self.expr(iter);
        dep
    }

    /// A value that becomes the function's result, or a part of it: `under`
    /// is the way down to the part.
    fn returned(&mut self, e: &Expr) {
        if self.result.is_empty() {
            self.expr(e);
            return;
        }
        self.flows(e, String::new());
    }

    fn flows(&mut self, e: &Expr, under: String) {
        let below = format!("{under}.");
        let targets: Vec<usize> = self
            .result
            .iter()
            .filter(|(p, _)| *p == under || p.starts_with(&below))
            .map(|(_, id)| *id)
            .collect();
        if targets.is_empty() {
            // A part of the result that holds no map.
            self.expr(e);
            return;
        }
        match e {
            Expr::Variable(..) | Expr::Field { .. } | Expr::Index { .. } => {
                match self.roots_of(e) {
                    Some(from) if from.len() == targets.len() => {
                        for (a, b) in from.into_iter().zip(targets) {
                            self.union(a, b);
                        }
                    }
                    _ => self.doubt(),
                }
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } if args.is_empty() && self.parsed.text(*method) == "clone" => {
                match self.roots_of(receiver) {
                    Some(from) if from.len() == targets.len() => {
                        for (a, b) in from.into_iter().zip(targets) {
                            self.union(a, b);
                        }
                    }
                    _ => self.doubt(),
                }
            }
            Expr::StructLit { fields, .. } => {
                for f in fields {
                    let field = self.sym(f.name);
                    let part = format!("{under}.{field}");
                    match &f.value {
                        Some(v) => self.flows(v, part),
                        None => {
                            let held = self.roots.get(&field).cloned();
                            let ours: Vec<usize> = self
                                .result
                                .iter()
                                .filter(|(p, _)| *p == part || p.starts_with(&format!("{part}.")))
                                .map(|(_, id)| *id)
                                .collect();
                            if ours.is_empty() {
                                continue;
                            }
                            match held {
                                Some(h) if h.len() == ours.len() => {
                                    for ((_, a), b) in h.into_iter().zip(ours) {
                                        self.union(a, b);
                                    }
                                }
                                _ => self.doubt(),
                            }
                        }
                    }
                }
            }
            e if self.map_made(e) => {}
            Expr::LitNull(_) => {}
            Expr::Block(..) | Expr::If { .. } | Expr::Tuple(..) => self.doubt(),
            other => {
                self.expr(other);
                self.doubt();
            }
        }
    }

    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::LitInt { .. }
            | Expr::LitStr { .. }
            | Expr::LitChar(..)
            | Expr::LitBool(..)
            | Expr::LitFloat(..)
            | Expr::LitNull(_)
            | Expr::Path(..)
            | Expr::Break(_)
            | Expr::Continue(_) => {}
            Expr::Variable(n, _) => {
                // A map used as a value: it goes somewhere unseen.
                if self.roots.contains_key(self.parsed.text(*n)) {
                    self.doubt();
                }
            }
            Expr::LitInterpolated { parts, .. } => {
                for p in parts {
                    if let FPart::Hole { expr, .. } = p {
                        self.expr(expr);
                    }
                }
            }
            Expr::Tuple(items, _) | Expr::ListLit { items, .. } => {
                for i in items {
                    self.expr(i);
                }
            }
            Expr::Block(b, _) | Expr::Unsafe(b, _) => self.block(b, false),
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.expr(cond);
                self.block(then_branch, false);
                if let Some(b) = else_branch {
                    self.block(b, false);
                }
            }
            Expr::Call {
                func, args, config, ..
            } => {
                self.expr(func);
                for a in args {
                    self.expr(a);
                }
                for c in config {
                    self.expr(&c.value);
                }
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
                config,
                ..
            } => {
                let name = self.parsed.text(*method).to_string();
                match self.place(receiver) {
                    Some(root) => {
                        if ADDS.contains(&name.as_str()) {
                            let dep = args
                                .iter()
                                .fold(Dep::default(), |all, a| all.join(self.deps(a)));
                            self.add(root, dep);
                        } else if !READS.contains(&name.as_str()) {
                            self.doubt();
                        }
                    }
                    None => self.field_or(receiver),
                }
                for a in args {
                    self.expr(a);
                }
                for c in config {
                    self.expr(&c.value);
                }
            }
            // A lookup, `m[k]`: reads the map and hands back a value.
            Expr::Index { base, index, .. } if self.place(base).is_some() => {
                self.expr(index);
            }
            Expr::Field { .. } | Expr::Index { .. } => self.field_or(e),
            Expr::SafeField { base, .. } => self.expr(base),
            Expr::Binary { op, lhs, rhs, .. } => {
                if matches!(op, BinaryOp::Eq | BinaryOp::Ne) {
                    match (self.roots_of(lhs), self.roots_of(rhs)) {
                        (Some(a), Some(b)) if a.len() == b.len() => {
                            for (a, b) in a.into_iter().zip(b) {
                                self.union(a, b);
                            }
                        }
                        (Some(_), None) => {
                            if !self.map_made(rhs) {
                                self.doubt();
                            }
                            self.expr(rhs);
                        }
                        (None, Some(_)) => {
                            if !self.map_made(lhs) {
                                self.doubt();
                            }
                            self.expr(lhs);
                        }
                        (Some(_), Some(_)) => self.doubt(),
                        (None, None) => {
                            self.expr(lhs);
                            self.expr(rhs);
                        }
                    }
                } else {
                    self.expr(lhs);
                    self.expr(rhs);
                }
            }
            Expr::Unary { expr, .. }
            | Expr::Cast { expr, .. }
            | Expr::Try(expr, _)
            | Expr::Throw(expr, _) => self.expr(expr),
            Expr::Coalesce {
                value, fallback, ..
            } => {
                self.expr(value);
                self.expr(fallback);
            }
            Expr::Range { start, end, .. } => {
                self.expr(start);
                self.expr(end);
            }
            Expr::Return(value, _) => {
                if let Some(v) = &**value {
                    self.returned(v);
                }
            }
            Expr::StructLit { fields, .. } => {
                for f in fields {
                    match &f.value {
                        Some(v) => self.expr(v),
                        None => {
                            if self.roots.contains_key(&self.sym(f.name)) {
                                self.doubt();
                            }
                        }
                    }
                }
            }
            // A `match`, a closure, a task, an `overlap`, a `catch`, a DSL,
            // `with`, assembly: names bound some way this does not follow.
            _ => self.doubt(),
        }
    }

    /// A field or an index read as a value: a map or a part of a parameter
    /// that holds some goes somewhere unseen; any other field is a value.
    fn field_or(&mut self, e: &Expr) {
        match e {
            Expr::Field { .. } | Expr::Index { .. } | Expr::Variable(..) => {
                match self.classify(e) {
                    Res::Not => {
                        // A field of a parameter that holds no map, or a
                        // chain that does not start at a name: the parts that
                        // are expressions have been visited by `locate`.
                        if self.locate_root_is_expr(e) {
                            self.expr_base(e);
                        }
                    }
                    Res::Map(_) | Res::Part => self.doubt(),
                }
            }
            other => self.expr(other),
        }
    }

    /// Whether a chain of fields starts at something that is not a name.
    fn locate_root_is_expr(&self, e: &Expr) -> bool {
        match e {
            Expr::Field { base, .. } | Expr::Index { base, .. } => self.locate_root_is_expr(base),
            Expr::Variable(..) => false,
            _ => true,
        }
    }

    fn expr_base(&mut self, e: &Expr) {
        match e {
            Expr::Field { base, .. } => self.expr_base(base),
            Expr::Index { base, index, .. } => {
                self.expr_base(base);
                self.expr(index);
            }
            other => self.expr(other),
        }
    }

    /// What a set of names comes to once the locals are followed: the
    /// parameters, or `None` where something is never trusted.
    fn resolve(&self, dep: &Dep) -> Option<BTreeSet<String>> {
        if dep.unknown {
            return None;
        }
        let mut out = BTreeSet::new();
        let mut seen = BTreeSet::new();
        let mut todo: Vec<String> = dep.names.iter().cloned().collect();
        while let Some(name) = todo.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            match self.locals.get(&name) {
                Some(local) => {
                    if local.unknown {
                        return None;
                    }
                    todo.extend(local.names.iter().cloned());
                }
                None => {
                    if self.params.contains(&name) {
                        out.insert(name);
                    } else {
                        return None;
                    }
                }
            }
        }
        Some(out)
    }
}

/// What a `pub` function says of its map positions.
#[derive(Debug, Clone)]
pub struct FnMaps {
    /// The ledger's key: `name`, or `Type::name` for a method.
    pub key: String,
    /// The lines of the `maps` key, sorted.
    pub lines: Vec<String>,
    pub shape: Shape,
}

/// How the lowering declares the hidden parameters of a `pub` function: one
/// per group of positions (ADR-010 D9, Part III 17.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shape {
    /// For each position, the first position of its group. Positions are
    /// numbered `self`'s first (a method), then the parameters in order, then
    /// the result.
    pub groups: Vec<usize>,
    /// For each position, the ordinal among its `impl` type's own parameters
    /// where it is a position of that type.
    pub within: Vec<Option<usize>>,
    /// How many positions are `self`'s: the written types start after them.
    pub start: usize,
    /// How many positions the parameters have (the result's follow).
    pub of_params: usize,
    /// The body was read to its end, so the groups are the joins it makes.
    /// Otherwise they are the positions of one written type.
    pub exact: bool,
    /// Two positions of the `impl`'s own type are joined: the type declares
    /// one parameter for each, so no signature can say it.
    pub conflict: bool,
}

impl Shape {
    /// The number of hidden parameters the function declares of its own.
    pub fn own(&self) -> usize {
        self.count(self.start, self.groups.len())
    }

    /// How many of them the parameters alone fix.
    pub fn fixed(&self) -> usize {
        self.count(self.start, self.start + self.of_params)
    }

    fn count(&self, from: usize, to: usize) -> usize {
        (from..to)
            .filter(|&at| self.within[at].is_none() && self.groups[at] == at)
            .count()
    }
}

/// The lines and the shape of one function, `None` where it has no map
/// position.
#[allow(clippy::too_many_arguments)]
fn lines(
    parsed: &Parsed,
    seeds: &Seeds,
    structs: &Structs,
    target: Option<&Type>,
    params: &[(String, &Type)],
    ret: Option<&Type>,
    body: Option<&Block>,
) -> Option<(Vec<String>, Shape)> {
    let target_name = target.map(|t| {
        let n = parsed.text(t.name);
        n.rsplit("::").next().unwrap_or(n)
    });
    let mut g = Gather {
        seeds,
        structs,
        target: target_name,
        nested: false,
    };
    let mut positions: Vec<Position> = Vec::new();
    let mut start = 0;
    let mut of_params = 0;
    for (name, ty) in params {
        let before = positions.len();
        let at = At {
            owner: name,
            label: name,
            scope: "",
            path: "",
            within: None,
        };
        g.of(parsed, ty, &at, &mut positions, 0);
        if name == SELF {
            start = positions.len() - before;
        } else {
            of_params += positions.len() - before;
        }
    }
    if let Some(ty) = ret {
        let at = At {
            owner: RESULT,
            label: RESULT,
            scope: "",
            path: "",
            within: None,
        };
        g.of(parsed, ty, &at, &mut positions, 0);
    }
    if positions.is_empty() {
        return None;
    }
    let mut b = Body {
        parsed,
        params: params.iter().map(|(n, _)| n.clone()).collect(),
        roots: BTreeMap::new(),
        root_name: Vec::new(),
        parent: Vec::new(),
        adds: Vec::new(),
        all_keyed: g.nested,
        locals: BTreeMap::new(),
        result: Vec::new(),
    };
    // One root per position: the ids are the positions' numbers.
    for p in &positions {
        let id = b.new_root(if p.owner == RESULT {
            None
        } else {
            Some(p.owner.clone())
        });
        if p.owner == RESULT {
            b.result.push((p.path.clone(), id));
        } else {
            b.roots
                .entry(p.owner.clone())
                .or_default()
                .push((p.path.clone(), id));
        }
    }
    match body {
        Some(block) => b.block(block, ret.is_some()),
        // A signature alone (a trait's): nothing is known of what goes in.
        None => b.doubt(),
    }
    let exact = !b.all_keyed;
    // The class of each root and what its members add.
    let mut how: BTreeMap<usize, Option<BTreeSet<String>>> = BTreeMap::new();
    for id in 0..b.parent.len() {
        let class = b.find(id);
        let dep = std::mem::take(&mut b.adds[id]);
        let seen = b.resolve(&dep);
        let entry = how.entry(class).or_insert_with(|| Some(BTreeSet::new()));
        *entry = match (entry.take(), seen) {
            (Some(mut all), Some(more)) => {
                all.extend(more);
                Some(all)
            }
            _ => None,
        };
    }
    let mut classes: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for (at, p) in positions.iter().enumerate() {
        classes.entry(b.find(at)).or_default().push(p.label.clone());
    }
    let mut out = Vec::new();
    for (class, mut labels) in classes {
        labels.sort();
        labels.dedup();
        let said = match (how.get(&class), b.all_keyed) {
            (_, true) | (None | Some(None), _) => "keyed".to_string(),
            (Some(Some(from)), false) => {
                format!(
                    "from({})",
                    from.iter().cloned().collect::<Vec<_>>().join(", ")
                )
            }
        };
        out.push(format!("{}: {said}", labels.join(" | ")));
    }
    out.sort();
    // The groups the lowering declares: the joins the body makes, or, where it
    // was not read to its end, the positions of one written type.
    let mut keys: Vec<String> = Vec::with_capacity(positions.len());
    for (at, p) in positions.iter().enumerate() {
        keys.push(if exact {
            format!("{}", b.find(at))
        } else {
            p.group.clone()
        });
    }
    // The first position of a group, but a position of the `impl`'s own type
    // first where there is one: its parameter is the type's, and the others
    // of the group take it.
    let mut first: BTreeMap<&String, usize> = BTreeMap::new();
    let mut conflict = false;
    for (at, key) in keys.iter().enumerate() {
        match first.get(key).copied() {
            None => {
                first.insert(key, at);
            }
            Some(canon) => match (positions[canon].within, positions[at].within) {
                (None, Some(_)) => {
                    first.insert(key, at);
                }
                (Some(a), Some(c)) if a != c => conflict = true,
                _ => {}
            },
        }
    }
    let groups: Vec<usize> = keys.iter().map(|k| first[k]).collect();
    Some((
        out,
        Shape {
            groups,
            within: positions.iter().map(|p| p.within).collect(),
            start,
            of_params,
            exact,
            conflict,
        },
    ))
}

/// Every `pub` function of the units that has a map position, with the key
/// the ledger files it under, its lines and its shape.
pub fn analyse(units: &[&Parsed]) -> Vec<FnMaps> {
    analyse_with(units, &Seeds::read(units))
}

/// The same, against the counts `seeds` has already read.
pub fn analyse_with(units: &[&Parsed], seeds: &Seeds) -> Vec<FnMaps> {
    let structs = Structs::read(units);
    let mut out = Vec::new();
    for parsed in units {
        for item in &parsed.program.items {
            match &item.node {
                Item::Fn {
                    name: Some(name),
                    args,
                    ret_type,
                    body,
                    is_public: true,
                    receiver: None,
                    ..
                } => {
                    let key = parsed.text(*name).to_string();
                    let params: Vec<(String, &Type)> = args
                        .iter()
                        .map(|a| (parsed.text(a.name).to_string(), &a.ty))
                        .collect();
                    if let Some((lines, shape)) = lines(
                        parsed,
                        seeds,
                        &structs,
                        None,
                        &params,
                        ret_type.as_ref(),
                        Some(body),
                    ) {
                        out.push(FnMaps { key, lines, shape });
                    }
                }
                Item::Impl {
                    target,
                    methods,
                    trait_name,
                    ..
                } => {
                    let target_name = parsed.text(target.name).to_string();
                    for method in methods {
                        let Item::Fn {
                            name: Some(name),
                            args,
                            ret_type,
                            body,
                            is_public: true,
                            receiver,
                            ..
                        } = &method.node
                        else {
                            continue;
                        };
                        let key = format!("{target_name}::{}", parsed.text(*name));
                        let mut params: Vec<(String, &Type)> = Vec::new();
                        if receiver.is_some() {
                            params.push((SELF.to_string(), target));
                        }
                        params.extend(
                            args.iter()
                                .map(|a| (parsed.text(a.name).to_string(), &a.ty)),
                        );
                        if let Some((lines, shape)) = lines(
                            parsed,
                            seeds,
                            &structs,
                            Some(target),
                            &params,
                            ret_type.as_ref(),
                            // A trait's method is a barrier (step 7): fixed
                            // keyed, whatever its body does.
                            if trait_name.is_some() {
                                None
                            } else {
                                Some(body)
                            },
                        ) {
                            out.push(FnMaps { key, lines, shape });
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Writes the lines into the ledger's `pub` functions.
pub fn infer(ledger: &mut Ledger, units: &[&Parsed]) {
    for f in analyse(units) {
        if let Some(contract) = ledger.functions.get_mut(&f.key) {
            contract.maps = f.lines;
        }
    }
}

/// The keys of the `pub` functions of a package that have a map position and
/// whose ledger line does not let a caller keep the fast hash: a line that
/// says `keyed`, or none.
pub fn keyed_functions(units: &[&Parsed], ledger: &Ledger, package: &str) -> Vec<String> {
    analyse(units)
        .into_iter()
        .map(|f| f.key)
        .filter(|key| {
            ledger
                .functions
                .get(&format!("{package}::{key}"))
                .is_none_or(|c| c.maps.is_empty() || c.maps.iter().any(|l| l.ends_with(": keyed")))
        })
        .collect()
}
