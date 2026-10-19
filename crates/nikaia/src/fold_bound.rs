// crates/nikaia/src/fold_bound.rs
//
// **A fold over a grammar's input takes no more steps than the input has room
// for; its counters are proved from that**
// ([ADR-367](../../docs/specification/adr/adr-367.md)).
//
// Three parts, each a decision of the record:
//
// * [`least_width`] (D1): the fewest bytes any match of a rule takes, read
//   from the grammar. A fold's step consumes at least `max(w, 1)` of them.
// * [`Extent`] (D2): whether the input's length is a fact when the fold
//   starts. It is for every input a grammar reads today, and is not for one
//   that arrives while it parses.
// * [`proven_counters`] (D3): the whole-number fields of the accumulator that
//   every step changes by at most `k`, outside any loop, and that the merge
//   leaves the sum of both partial results' fields or one of them. The
//   invariant `|f| <= k * steps` is checked by the solver before it is used
//   (ADR-361 D4), and a field's checks are proved where `k * 2^60 / max(w, 1)`
//   fits its type.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{GrammarDef, GrammarRule, Item, Pattern, Repeat};
use crate::parser::Parsed;

// ---------------------------------------------------------------------------
// D1: the least width of a rule
// ---------------------------------------------------------------------------

/// The bytes a literal written in the source matches: an escape is one byte,
/// a `\u{…}` at least one, which only ever counts fewer than it matches.
fn literal_bytes(written: &str) -> u64 {
    let mut chars = written.chars();
    let mut bytes = 0u64;
    while let Some(c) = chars.next() {
        if c != '\\' {
            bytes += c.len_utf8() as u64;
            continue;
        }
        bytes += 1;
        if chars.next() == Some('u') {
            for d in chars.by_ref() {
                if d == '}' {
                    break;
                }
            }
        }
    }
    bytes
}

/// What D1 reads of one grammar: its rules by name, and the frame boundary a
/// `frame_end` stands for.
struct Widths<'a> {
    parsed: &'a Parsed,
    rules: BTreeMap<String, &'a GrammarRule>,
    known: BTreeMap<String, u64>,
    open: BTreeSet<String>,
}

impl<'a> Widths<'a> {
    fn rule(&mut self, name: &str, boundary: u64) -> u64 {
        if let Some(w) = self.known.get(name) {
            return *w;
        }
        // **A rule that reaches itself is taken at nothing for the recursive
        // call** (D1): it can only make the width smaller.
        if !self.open.insert(name.to_string()) {
            return 0;
        }
        let Some(rule) = self.rules.get(name).copied() else {
            self.open.remove(name);
            return 0;
        };
        let w = rule
            .alts
            .iter()
            .map(|alt| self.pattern(&alt.pattern.node, boundary))
            .min()
            .unwrap_or(0);
        self.open.remove(name);
        // Only a width found with nothing open is the rule's own: one found
        // inside a cycle took a recursive call at nothing on the way.
        if self.open.is_empty() {
            self.known.insert(name.to_string(), w);
        }
        w
    }

    fn pattern(&mut self, pattern: &Pattern, boundary: u64) -> u64 {
        match pattern {
            Pattern::Literal(text) => literal_bytes(text),
            Pattern::Seq(parts) => parts.iter().fold(0u64, |sum, p| {
                sum.saturating_add(self.pattern(&p.node, boundary))
            }),
            Pattern::Choice(branches) => branches
                .iter()
                .map(|p| self.pattern(&p.node, boundary))
                .min()
                .unwrap_or(0),
            Pattern::Bind { pat, .. } | Pattern::Group(pat) => self.pattern(&pat.node, boundary),
            Pattern::Repeat { pat, rep } => {
                let times = match rep {
                    Repeat::Star | Repeat::Optional => 0,
                    Repeat::Plus => 1,
                    Repeat::Exactly(m) | Repeat::AtLeast(m) | Repeat::Between(m, _) => {
                        u64::from(*m)
                    }
                };
                match times {
                    0 => 0,
                    _ => self.pattern(&pat.node, boundary).saturating_mul(times),
                }
            }
            Pattern::Cut | Pattern::Fold(_) => 0,
            Pattern::Ref { name, args, .. } => {
                let called = self.parsed.text(*name).to_string();
                if self.rules.contains_key(&called) {
                    return self.rule(&called, boundary);
                }
                match (called.as_str(), args.len()) {
                    // A class matches one character, which takes at least a
                    // byte.
                    ("digit" | "hex_digit" | "any" | "char" | "alpha1" | "multispace1", 0) => 1,
                    ("frame_end", 0) => boundary,
                    // What `p` takes, read as a number or as bytes.
                    ("dec" | "bytes", 1..) => self.pattern(&args[0].node, boundary),
                    // Any other built-in may match nothing, as far as D1
                    // reads it: counting less is never wrong.
                    _ => 0,
                }
            }
        }
    }
}

/// **The least width of `rule` in `grammar`** (ADR-367 D1): the fewest bytes
/// any match of it takes. A `frame_end` inside it is the boundary of the
/// rule's own `@frame`, and nothing where it has none. `None` where the
/// grammar or the rule is not in the program.
pub fn least_width(parsed: &Parsed, grammar: &str, rule: &str) -> Option<u64> {
    parsed
        .program
        .items
        .iter()
        .find_map(|item| match &item.node {
            Item::Grammar(g) if parsed.text(g.name) == grammar => rule_width(parsed, g, rule),
            _ => None,
        })
}

/// [`least_width`] of a rule of a grammar at hand.
fn rule_width(parsed: &Parsed, grammar: &GrammarDef, rule: &str) -> Option<u64> {
    let rules: BTreeMap<String, &GrammarRule> = grammar
        .rules
        .iter()
        .map(|r| (parsed.text(r.name).to_string(), r))
        .collect();
    let element = *rules.get(rule)?;
    let boundary = element
        .frame
        .as_ref()
        .and_then(|f| f.boundary.as_deref())
        .map(literal_bytes)
        .unwrap_or(0);
    let mut widths = Widths {
        parsed,
        rules,
        known: BTreeMap::new(),
        open: BTreeSet::new(),
    };
    Some(widths.rule(rule, boundary))
}

// ---------------------------------------------------------------------------
// D2: the input's extent
// ---------------------------------------------------------------------------

/// The least `len` below which every input of known extent stays: the length
/// of every text and every `Bytes` is below 2^60 (Part III, `optimization`).
pub const LENGTH_BOUND: u128 = 1 << 60;

/// **Whether a fold's input has a length when the fold starts**
/// ([ADR-367](../../docs/specification/adr/adr-367.md) D2).
///
/// A fold's step count, and every proof of D3 built on it, starts from this.
///
/// **WARNING, for whoever adds a kind of input.** Every input a grammar reads
/// today is held whole before the parse starts - text (a `String` or a
/// mapping) or a `Bytes` - and is [`Extent::Known`]. A grammar over input
/// that **arrives while it parses** (winnow's `Partial`, a stream, a socket)
/// must give [`Extent::Unbounded`]: its length is not known when the fold
/// starts and has no bound, so no step count exists and no counter may be
/// proved from one. **A new kind of input starts as `Unbounded`** until a
/// record decides otherwise; making it `Known` without one would remove
/// overflow checks a stream needs, and an overflow would then run on silently
/// into a wrong result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extent {
    /// Held whole before the parse: its length is a value below
    /// [`LENGTH_BOUND`].
    Known,
    /// Arrives while it is parsed: no length, no bound, no step count.
    Unbounded,
}

/// What a grammar's entry reads. Today every entry takes text or `Bytes`
/// (ADR-366 D1), both held whole.
///
/// **ADR-367 D2: this is the one place that decides the extent.** An entry
/// that reads input arriving while it parses (a winnow `Partial`, a stream)
/// must answer [`Extent::Unbounded`] here, and so must any kind of input this
/// function does not know yet: there is no step count over a stream.
pub fn input_extent(_grammar: &GrammarDef) -> Extent {
    // No grammar reads a stream today (ADR-366 D1): text and `Bytes` only.
    // When one does, its entry is told apart here and gives `Unbounded`.
    Extent::Known
}

/// **The most steps a fold over an input of this extent takes** (D1, D2):
/// `len / max(w, 1)` with `len` below [`LENGTH_BOUND`], and nothing - no
/// proof - where the extent is unbounded.
pub fn most_steps(extent: Extent, width: u64) -> Option<u128> {
    match extent {
        Extent::Known => Some((LENGTH_BOUND - 1) / u128::from(width.max(1))),
        // ADR-367 D2: a stream has no length, so no step count.
        Extent::Unbounded => None,
    }
}

// ---------------------------------------------------------------------------
// D3: growth per step
// ---------------------------------------------------------------------------

use crate::ast::{BinaryOp, Block, Expr, FoldSpec, Spanned, Stmt, Type, UnaryOp};
use crate::contracts::sync::{visit_expr, visit_expr_blocks, visit_stmt, visit_stmt_blocks};
use nikaia_std::tools::prover_arena::TermArena;

/// A type as D3 reads it: the last segment of its name and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ty {
    name: String,
    args: Vec<Ty>,
}

impl Ty {
    fn of(parsed: &Parsed, ty: &Type) -> Ty {
        let written = parsed.text(ty.name);
        Ty {
            name: written.rsplit("::").next().unwrap_or(written).to_string(),
            args: ty.generics.iter().map(|g| Ty::of(parsed, g)).collect(),
        }
    }

    fn named(name: &str) -> Ty {
        Ty {
            name: name.to_string(),
            args: Vec::new(),
        }
    }

    fn is_map(&self) -> bool {
        (self.name == "HashMap" || self.name == "BTreeMap") && self.args.len() == 2
    }
}

/// The least and the greatest value of a whole-number type, where D3 proves
/// one.
fn range_of(ty: &str) -> Option<(i128, i128)> {
    Some(match ty {
        "i8" => (i8::MIN.into(), i8::MAX.into()),
        "i16" => (i16::MIN.into(), i16::MAX.into()),
        "i32" => (i32::MIN.into(), i32::MAX.into()),
        "i64" | "isize" => (i64::MIN.into(), i64::MAX.into()),
        "u8" => (0, u8::MAX.into()),
        "u16" => (0, u16::MAX.into()),
        "u32" => (0, u32::MAX.into()),
        "u64" | "usize" => (0, u64::MAX.into()),
        _ => return None,
    })
}

/// A function or method of the program, by the byte its item starts at.
struct FnRef<'a> {
    item: &'a Spanned<Item>,
    /// The type of `self` and of a constructor's result: the `impl`'s.
    owner: Option<Ty>,
}

/// A struct's fields: name, type and default.
type Fields<'a> = Vec<(String, Ty, Option<&'a Expr>)>;

/// What D3 looks up in the program.
struct Index<'a> {
    parsed: &'a Parsed,
    /// Each struct's fields, with their type and default.
    structs: BTreeMap<String, Fields<'a>>,
    /// Methods by `(type, name)`, constructors under the name `""`.
    methods: BTreeMap<(String, String), Vec<usize>>,
    /// Every `impl`'s type.
    impls: BTreeSet<String>,
    /// Methods by name alone, for a receiver of no known type.
    by_name: BTreeMap<String, Vec<usize>>,
    free: BTreeMap<String, usize>,
    fns: BTreeMap<usize, FnRef<'a>>,
    /// What a name written in a call or a path may reach.
    reached_by: BTreeMap<String, BTreeSet<usize>>,
    /// The methods of a trait's `impl`, which an operator, a conversion or
    /// `std` may run where no call names them.
    implicit: BTreeSet<usize>,
}

impl<'a> Index<'a> {
    fn of(parsed: &'a Parsed) -> Index<'a> {
        let mut ix = Index {
            parsed,
            structs: BTreeMap::new(),
            methods: BTreeMap::new(),
            impls: BTreeSet::new(),
            by_name: BTreeMap::new(),
            free: BTreeMap::new(),
            fns: BTreeMap::new(),
            reached_by: BTreeMap::new(),
            implicit: BTreeSet::new(),
        };
        for item in &parsed.program.items {
            match &item.node {
                Item::Struct { name, fields, .. } => {
                    let fields = fields
                        .iter()
                        .map(|f| {
                            (
                                parsed.text(f.name).to_string(),
                                Ty::of(parsed, &f.ty),
                                f.default.as_ref().map(|d| &d.value),
                            )
                        })
                        .collect();
                    ix.structs.insert(parsed.text(*name).to_string(), fields);
                }
                Item::Fn {
                    name: Some(name), ..
                } => {
                    let at = item.span.start as usize;
                    let name = parsed.text(*name).to_string();
                    ix.free.insert(name.clone(), at);
                    ix.reached_by.entry(name).or_default().insert(at);
                    ix.fns.insert(at, FnRef { item, owner: None });
                }
                Item::Impl {
                    trait_name,
                    target,
                    methods,
                } => {
                    let owner = Ty::of(parsed, target);
                    ix.impls.insert(owner.name.clone());
                    for method in methods {
                        let Item::Fn { name, .. } = &method.node else {
                            continue;
                        };
                        let at = method.span.start as usize;
                        let called = name.map(|n| parsed.text(n).to_string()).unwrap_or_default();
                        ix.methods
                            .entry((owner.name.clone(), called.clone()))
                            .or_default()
                            .push(at);
                        let reached = match called.is_empty() {
                            true => owner.name.clone(),
                            false => {
                                ix.by_name.entry(called.clone()).or_default().push(at);
                                called
                            }
                        };
                        ix.reached_by.entry(reached).or_default().insert(at);
                        if trait_name.is_some() {
                            ix.implicit.insert(at);
                        }
                        ix.fns.insert(
                            at,
                            FnRef {
                                item: method,
                                owner: Some(owner.clone()),
                            },
                        );
                    }
                }
                _ => {}
            }
        }
        ix
    }

    fn text(&self, sym: winnow_grammar::Symbol) -> &'a str {
        self.parsed.text(sym)
    }

    /// The whole-number fields of a struct.
    fn whole_fields(&self, name: &str) -> Vec<(String, Option<&'a Expr>)> {
        self.structs
            .get(name)
            .map(|fields| {
                fields
                    .iter()
                    .filter(|(_, ty, _)| ty.args.is_empty() && range_of(&ty.name).is_some())
                    .map(|(f, _, d)| (f.clone(), *d))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn field_type(&self, of: &Ty, field: &str) -> Option<Ty> {
        self.structs
            .get(&of.name)?
            .iter()
            .find(|(f, _, _)| f == field)
            .map(|(_, ty, _)| ty.clone())
    }

    /// The methods a call of `method` on a value of type `ty` may run: the
    /// type's own where it is one of the program's, none where it is
    /// another's, and every method of that name where the type is not known.
    fn methods_of(&self, ty: Option<&Ty>, method: &str) -> Vec<usize> {
        match ty {
            Some(t) if self.impls.contains(&t.name) => self
                .methods
                .get(&(t.name.clone(), method.to_string()))
                .cloned()
                .unwrap_or_default(),
            Some(_) => Vec::new(),
            None => self.by_name.get(method).cloned().unwrap_or_default(),
        }
    }

    /// The functions a callee written as `func` may run.
    fn callees(&self, func: &Expr) -> Vec<usize> {
        match func {
            Expr::Variable(name, _) => {
                let name = self.text(*name);
                match self.impls.contains(name) || self.structs.contains_key(name) {
                    true => self
                        .methods
                        .get(&(name.to_string(), String::new()))
                        .cloned()
                        .unwrap_or_default(),
                    false => self.free.get(name).map(|at| vec![*at]).unwrap_or_default(),
                }
            }
            Expr::Path(segments, _) if segments.len() >= 2 => {
                let ty = self.text(segments[segments.len() - 2]);
                let name = self.text(segments[segments.len() - 1]);
                self.methods
                    .get(&(ty.to_string(), name.to_string()))
                    .cloned()
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }
}

/// The amount a compound assignment adds or takes, where it is a literal.
fn literal_amount(value: &Expr) -> Option<(u128, bool)> {
    match value {
        Expr::LitInt {
            value, negative, ..
        } => Some((u128::from(*value), *negative)),
        Expr::Unary {
            op: UnaryOp::Neg,
            expr,
            ..
        } => match &**expr {
            Expr::LitInt { value, .. } => Some((u128::from(*value), true)),
            _ => None,
        },
        _ => None,
    }
}

fn field_name(target: &Expr) -> Option<winnow_grammar::Symbol> {
    match target {
        Expr::Field { name, .. } | Expr::SafeField { name, .. } => Some(*name),
        _ => None,
    }
}

/// **The walk of one part of a fold** (D3): what each run of it changes each
/// whole-number field by, through the methods it calls, the lambdas a map
/// entry runs, and the constructors it builds with. Its own walk, not a result
/// summary (#641).
struct Walk<'i, 'a> {
    ix: &'i Index<'a>,
    /// By field name: the most one run adds to or takes from it, `None` where
    /// a write has no bound or stands in a loop.
    k: BTreeMap<String, Option<u128>>,
    /// Fields some write lowers, or a literal starts below zero.
    lowered: BTreeSet<String>,
    /// The compound assignments to a field, by the statement's first byte.
    sites: BTreeMap<String, Vec<i64>>,
    stack: Vec<usize>,
    reached: BTreeSet<usize>,
    /// Something the walk cannot follow: nothing of this fold is proved.
    broken: bool,
    /// The lambdas met, by their body: whether they run at most once per
    /// run of what holds them, and the types of their parameters.
    lambdas: BTreeMap<usize, (bool, Vec<(String, Ty)>)>,
    /// The lambda bodies walked, which the block walk then skips.
    walked: BTreeSet<usize>,
    /// The callees met in call position, not as values, by node.
    called: BTreeSet<u32>,
}

type Env = BTreeMap<String, Ty>;

impl<'i, 'a> Walk<'i, 'a> {
    fn new(ix: &'i Index<'a>) -> Walk<'i, 'a> {
        Walk {
            ix,
            k: BTreeMap::new(),
            lowered: BTreeSet::new(),
            sites: BTreeMap::new(),
            stack: Vec::new(),
            reached: BTreeSet::new(),
            broken: false,
            lambdas: BTreeMap::new(),
            walked: BTreeSet::new(),
            called: BTreeSet::new(),
        }
    }

    fn poison(&mut self, field: &str) {
        self.k.insert(field.to_string(), None);
    }

    fn grow(&mut self, field: &str, amount: u128, negative: bool) {
        if negative && amount > 0 {
            self.lowered.insert(field.to_string());
        }
        let slot = self.k.entry(field.to_string()).or_insert(Some(0));
        *slot = slot.and_then(|k| k.checked_add(amount));
    }

    fn function(&mut self, at: usize, looped: bool) {
        if self.stack.contains(&at) {
            // A function that reaches itself runs an unknown number of times.
            self.broken = true;
            return;
        }
        let Some(fun) = self.ix.fns.get(&at) else {
            return;
        };
        let Item::Fn {
            receiver,
            args,
            body,
            ..
        } = &fun.item.node
        else {
            return;
        };
        self.reached.insert(at);
        self.stack.push(at);
        let mut env = Env::new();
        if let (Some(_), Some(owner)) = (receiver, &fun.owner) {
            env.insert("self".to_string(), owner.clone());
        }
        for arg in args {
            env.insert(
                self.ix.text(arg.name).to_string(),
                Ty::of(self.ix.parsed, &arg.ty),
            );
        }
        self.block(body, &env, looped);
        self.stack.pop();
    }

    fn type_of(&self, expr: &Expr, env: &Env) -> Option<Ty> {
        match expr {
            Expr::Variable(name, _) => env.get(self.ix.text(*name)).cloned(),
            Expr::Field { base, name, .. } => {
                let base = self.type_of(base, env)?;
                self.ix.field_type(&base, self.ix.text(*name))
            }
            Expr::Unary {
                op: UnaryOp::Ref,
                expr,
                ..
            } => self.type_of(expr, env),
            Expr::StructLit { name, .. } => Some(Ty::named(self.ix.text(*name))),
            Expr::Call { func, .. } => match &**func {
                Expr::Variable(name, _) if self.ix.structs.contains_key(self.ix.text(*name)) => {
                    Some(Ty::named(self.ix.text(*name)))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// The value type of the map whose `entry` a chain of entry calls starts
    /// from.
    fn entry_value(&self, mut expr: &Expr, env: &Env) -> Option<Ty> {
        loop {
            let Expr::MethodCall {
                receiver, method, ..
            } = expr
            else {
                return None;
            };
            match self.ix.text(*method) {
                "entry" => {
                    let map = self.type_of(receiver, env)?;
                    return map.is_map().then(|| map.args[1].clone());
                }
                "and_modify" | "or_insert" | "or_insert_with" | "or_default" => expr = receiver,
                _ => return None,
            }
        }
    }

    fn block(&mut self, block: &Block, env: &Env, looped: bool) {
        let mut env = env.clone();
        for stmt in &block.stmts {
            self.stmt(stmt, &mut env, looped);
        }
    }

    fn stmt(&mut self, stmt: &Spanned<Stmt>, env: &mut Env, looped: bool) {
        let parsed = self.ix.parsed;
        // A `while`'s condition runs as often as its body.
        let here = looped || matches!(stmt.node, Stmt::While { .. });
        if let Stmt::Assign { target, op, value } = &stmt.node {
            self.write(target, op.as_ref(), value, stmt.span.start as i64, here);
        }
        {
            let shown = env.clone();
            visit_stmt(parsed, &stmt.node, &mut |e| self.expr(e, &shown, here));
        }
        match &stmt.node {
            Stmt::Let {
                names, ty, value, ..
            } => {
                for name in names {
                    env.remove(parsed.text(*name));
                }
                if names.len() == 1 {
                    let ty = match ty {
                        Some(t) => Some(Ty::of(parsed, t)),
                        None => self.type_of(value, env),
                    };
                    if let Some(ty) = ty {
                        env.insert(parsed.text(names[0]).to_string(), ty);
                    }
                }
            }
            Stmt::For { bindings, .. } => {
                for name in bindings {
                    env.remove(parsed.text(*name));
                }
            }
            _ => {}
        }
        let loop_body = match &stmt.node {
            Stmt::For { body, .. } | Stmt::While { body, .. } => {
                Some(body as *const Block as usize)
            }
            _ => None,
        };
        let shown = env.clone();
        visit_stmt_blocks(&stmt.node, &mut |b| self.nested(b, &shown, here, loop_body));
    }

    fn nested(&mut self, b: &Block, env: &Env, looped: bool, loop_body: Option<usize>) {
        let at = b as *const Block as usize;
        // A lambda's body is walked where the lambda stands.
        if self.walked.contains(&at) {
            return;
        }
        self.block(b, env, looped || loop_body == Some(at));
    }

    /// An expression on its own, as a fold's `init` is.
    fn expression(&mut self, expr: &Expr, env: &Env, looped: bool) {
        let parsed = self.ix.parsed;
        visit_expr(parsed, expr, &mut |e| self.expr(e, env, looped));
        visit_expr_blocks(expr, &mut |b| self.nested(b, env, looped, None));
    }

    fn write(&mut self, target: &Expr, op: Option<&BinaryOp>, value: &Expr, at: i64, looped: bool) {
        let Some(name) = field_name(target) else {
            return;
        };
        let field = self.ix.text(name);
        match (op, literal_amount(value), looped) {
            (Some(op @ (BinaryOp::Add | BinaryOp::Sub)), Some((amount, negative)), false) => {
                let lowers = negative != (*op == BinaryOp::Sub);
                self.grow(field, amount, lowers);
                if let Expr::Field { base, .. } = target
                    && matches!(**base, Expr::Variable(..))
                {
                    self.sites.entry(field.to_string()).or_default().push(at);
                }
            }
            _ => self.poison(field),
        }
    }

    /// A field handed to a callee that may change it in place, or to one the
    /// walk does not know.
    fn arguments(&mut self, args: &[Expr], callees: &[usize]) {
        for (i, arg) in args.iter().enumerate() {
            let Some(name) = field_name(arg) else {
                continue;
            };
            let kept = !callees.is_empty()
                && callees
                    .iter()
                    .all(|at| match self.ix.fns.get(at).map(|f| &f.item.node) {
                        Some(Item::Fn { args, .. }) => args.get(i).is_some_and(|a| !a.mutable),
                        _ => false,
                    });
            if !kept {
                self.poison(self.ix.text(name));
            }
        }
    }

    fn expr(&mut self, expr: &Expr, env: &Env, looped: bool) {
        let ix = self.ix;
        match expr {
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            }
            | Expr::SafeMethod {
                receiver,
                method,
                args,
                ..
            } => {
                let name = ix.text(*method);
                let once = matches!(name, "and_modify" | "or_insert_with" | "or_insert_with_key");
                for arg in args {
                    if let Expr::Closure { params, body, .. } = arg {
                        let at = body as *const Block as usize;
                        let mut bound = Vec::new();
                        if name == "and_modify"
                            && params.len() == 1
                            && let Some(value) = self.entry_value(expr, env)
                        {
                            bound.push((ix.text(params[0]).to_string(), value));
                        }
                        self.lambdas.entry(at).or_insert((once, bound));
                    }
                }
                let callees = ix.methods_of(self.type_of(receiver, env).as_ref(), name);
                self.arguments(args, &callees);
                for at in callees {
                    self.function(at, looped);
                }
            }
            Expr::Call { func, args, .. } => {
                self.called.insert(crate::ast::id_of(func));
                let callees = ix.callees(func);
                self.arguments(args, &callees);
                for at in callees {
                    self.function(at, looped);
                }
            }
            // A function named as a value runs where the walk cannot count it.
            Expr::Variable(..) | Expr::Path(..) => {
                if self.called.contains(&crate::ast::id_of(expr)) {
                    return;
                }
                if let Expr::Variable(name, _) = expr
                    && env.contains_key(ix.text(*name))
                {
                    return;
                }
                for at in ix.callees(expr) {
                    self.function(at, true);
                }
            }
            // **A lambda is walked where it stands**, also inside an
            // f-string's hole, which no block walk reaches. One that is not
            // known to run at most once may run any number of times.
            Expr::Closure { body, .. } => {
                let at = body as *const Block as usize;
                let (once, params) = self.lambdas.remove(&at).unwrap_or((false, Vec::new()));
                self.walked.insert(at);
                let mut inner = env.clone();
                for (name, ty) in params {
                    inner.insert(name, ty);
                }
                self.block(body, &inner, looped || !once);
            }
            Expr::StructLit { name, fields, .. } => {
                let ty = match ix.text(*name) {
                    "Self" => env.get("self").map(|t| t.name.clone()).unwrap_or_default(),
                    other => other.to_string(),
                };
                for (field, default) in ix.whole_fields(&ty) {
                    let value = match fields.iter().find(|f| ix.text(f.name) == field) {
                        Some(init) => init.value.as_ref(),
                        None => default,
                    };
                    match (value.and_then(literal_amount), looped) {
                        (Some((amount, negative)), false) => self.grow(&field, amount, negative),
                        _ => self.poison(&field),
                    }
                }
            }
            Expr::With { fields, .. } => {
                for f in fields {
                    self.poison(ix.text(f.name));
                }
            }
            Expr::Unary {
                op: UnaryOp::Ref,
                expr: inner,
                ..
            } => {
                if let Some(name) = field_name(inner) {
                    self.poison(ix.text(name));
                }
            }
            // What a `spawn` runs, runs beside the fold.
            Expr::Spawn { .. } => self.broken = true,
            _ => {}
        }
    }
}

/// **What the merge leaves each field** (D3): the sum of both partial
/// results' fields (`self.f += other.f`, once), or one of them. A merge that
/// walks the other result's map merges each of its entries into the entry of
/// the same key, once: the keys of a map are distinct.
struct Merge<'i, 'a> {
    ix: &'i Index<'a>,
    sites: BTreeMap<String, Vec<i64>>,
    /// Fields the merge changes in any other way.
    bad: BTreeSet<String>,
    reached: BTreeSet<usize>,
    broken: bool,
    stack: Vec<usize>,
}

impl<'i, 'a> Merge<'i, 'a> {
    fn function(&mut self, at: usize) {
        if self.stack.contains(&at) {
            self.broken = true;
            return;
        }
        let Some(fun) = self.ix.fns.get(&at) else {
            self.broken = true;
            return;
        };
        let (
            Item::Fn {
                receiver: Some(_),
                args,
                body,
                ..
            },
            Some(owner),
        ) = (&fun.item.node, &fun.owner)
        else {
            self.broken = true;
            return;
        };
        if args.len() != 1 {
            self.broken = true;
            return;
        }
        self.reached.insert(at);
        self.stack.push(at);
        let other = self.ix.text(args[0].name);
        let mut summed: BTreeSet<String> = BTreeSet::new();
        for stmt in &body.stmts {
            // `self` and the other result stay what the merge was handed.
            let rebinds = match &stmt.node {
                Stmt::Assign {
                    target: Expr::Variable(name, _),
                    ..
                } => [other, "self"].contains(&self.ix.text(*name)),
                Stmt::Let { names, .. }
                | Stmt::For {
                    bindings: names, ..
                } => names
                    .iter()
                    .any(|n| [other, "self"].contains(&self.ix.text(*n))),
                _ => false,
            };
            if rebinds {
                self.broken = true;
            }
            if let Some(field) = self.sum_of_both(&stmt.node, other) {
                if !summed.insert(field.clone()) {
                    self.bad.insert(field.clone());
                }
                self.sites
                    .entry(field)
                    .or_default()
                    .push(stmt.span.start as i64);
                continue;
            }
            if self.entry_wise(&stmt.node, owner, other) {
                continue;
            }
            // Anything else: every field it changes is not merged as D3
            // reads a merge.
            let mut walk = Walk::new(self.ix);
            let mut env = Env::new();
            env.insert("self".to_string(), owner.clone());
            env.insert(other.to_string(), Ty::of(self.ix.parsed, &args[0].ty));
            walk.stmt(stmt, &mut env, false);
            self.bad.extend(walk.k.keys().cloned());
            self.reached.extend(walk.reached);
            self.broken |= walk.broken;
        }
        self.stack.pop();
    }

    /// `self.f += other.f`.
    fn sum_of_both(&self, stmt: &Stmt, other: &str) -> Option<String> {
        let Stmt::Assign {
            target: Expr::Field {
                base: into, name, ..
            },
            op: Some(BinaryOp::Add),
            value:
                Expr::Field {
                    base: from,
                    name: read,
                    ..
                },
        } = stmt
        else {
            return None;
        };
        let named = |e: &Expr, n: &str| matches!(e, Expr::Variable(v, _) if self.ix.text(*v) == n);
        let field = self.ix.text(*name);
        (named(into, "self") && named(from, other) && self.ix.text(*read) == field)
            .then(|| field.to_string())
    }

    /// `for (key, value) in other.m.drain() { self.n.entry(key)
    /// .and_modify fn (into) { into.merge(ref value) }.or_insert(value) }`:
    /// each entry of the other map merged once into the entry of its key.
    fn entry_wise(&mut self, stmt: &Stmt, owner: &Ty, other: &str) -> bool {
        let ix = self.ix;
        let named = |e: &Expr, n: &str| matches!(e, Expr::Variable(v, _) if ix.text(*v) == n);
        let Stmt::For {
            bindings,
            iter,
            body,
            ..
        } = stmt
        else {
            return false;
        };
        if bindings.len() != 2 || body.stmts.len() != 1 {
            return false;
        }
        let (key, value) = (ix.text(bindings[0]), ix.text(bindings[1]));
        let source = match iter {
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } if args.is_empty() && matches!(ix.text(*method), "drain" | "into_iter") => {
                &**receiver
            }
            other => other,
        };
        let Expr::Field { base, name, .. } = source else {
            return false;
        };
        if !named(base, other)
            || !ix
                .field_type(owner, ix.text(*name))
                .is_some_and(|t| t.is_map())
        {
            return false;
        }
        let Stmt::Expr(first) = &body.stmts[0].node else {
            return false;
        };
        let mut chain: &Expr = first;
        let mut modify: Option<(&str, &Block)> = None;
        let mut inserted = false;
        let target = loop {
            let Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } = chain
            else {
                return false;
            };
            match (ix.text(*method), args.as_slice()) {
                ("and_modify", [Expr::Closure { params, body, .. }])
                    if modify.is_none() && params.len() == 1 =>
                {
                    modify = Some((ix.text(params[0]), body));
                }
                ("or_insert", [moved]) if !inserted && named(moved, value) => inserted = true,
                ("entry", [k]) if named(k, key) => break &**receiver,
                _ => return false,
            }
            chain = &**receiver;
        };
        let Expr::Field { base, name, .. } = target else {
            return false;
        };
        let Some(map) = ix.field_type(owner, ix.text(*name)).filter(|t| t.is_map()) else {
            return false;
        };
        if !named(base, "self") {
            return false;
        }
        let Some((into, merged)) = modify else {
            return true;
        };
        let [
            Spanned {
                node:
                    Stmt::Expr(Expr::MethodCall {
                        receiver,
                        method,
                        args,
                        ..
                    }),
                ..
            },
        ] = merged.stmts.as_slice()
        else {
            return false;
        };
        let handed = match args.as_slice() {
            [
                Expr::Unary {
                    op: UnaryOp::Ref,
                    expr,
                    ..
                },
            ] => &**expr,
            [plain] => plain,
            _ => return false,
        };
        if !named(receiver, into) || !named(handed, value) {
            return false;
        }
        let callees = ix.methods_of(Some(&map.args[1]), ix.text(*method));
        let [callee] = callees.as_slice() else {
            return false;
        };
        self.function(*callee);
        true
    }
}

/// Who holds a piece of code, for D3's question whether a method runs only
/// inside the fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Owner {
    Function(usize),
    /// A fold's `init`, step and merge, by the rule's first byte.
    Fold(usize),
    Elsewhere,
}

/// What each owner names and which fields it changes, read from every piece
/// of code in the program.
#[derive(Default)]
struct Uses {
    names: BTreeMap<Owner, BTreeSet<String>>,
    touches: BTreeMap<String, BTreeSet<Owner>>,
}

impl Uses {
    fn of(ix: &Index<'_>) -> Uses {
        let mut uses = Uses::default();
        let parsed = ix.parsed;
        for item in &parsed.program.items {
            match &item.node {
                Item::Fn { body, .. } => {
                    uses.block(ix, body, Owner::Function(item.span.start as usize))
                }
                Item::Impl { methods, .. } => {
                    for method in methods {
                        if let Item::Fn { body, .. } = &method.node {
                            uses.block(ix, body, Owner::Function(method.span.start as usize));
                        }
                    }
                }
                Item::Test { body, .. } | Item::Bench { body, .. } => {
                    uses.block(ix, body, Owner::Elsewhere)
                }
                Item::Comptime { value, .. } => uses.expression(ix, value, Owner::Elsewhere),
                Item::Struct { fields, .. } => {
                    for f in fields {
                        if let Some(d) = &f.default {
                            uses.expression(ix, &d.value, Owner::Elsewhere);
                        }
                    }
                }
                Item::Grammar(g) => {
                    for rule in &g.rules {
                        for alt in &rule.alts {
                            uses.pattern(
                                ix,
                                &alt.pattern.node,
                                Owner::Fold(rule.span.start as usize),
                            );
                            if let Some(action) = &alt.action {
                                uses.block(ix, action, Owner::Elsewhere);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        uses
    }

    fn pattern(&mut self, ix: &Index<'_>, pattern: &Pattern, owner: Owner) {
        match pattern {
            Pattern::Seq(parts) | Pattern::Choice(parts) => {
                parts.iter().for_each(|p| self.pattern(ix, &p.node, owner))
            }
            Pattern::Bind { pat, .. } | Pattern::Group(pat) | Pattern::Repeat { pat, .. } => {
                self.pattern(ix, &pat.node, owner)
            }
            Pattern::Ref { args, .. } => args.iter().for_each(|p| self.pattern(ix, &p.node, owner)),
            Pattern::Fold(spec) => {
                self.expression(ix, &spec.init, owner);
                self.expression(ix, &spec.step, owner);
                if let Some(merge) = &spec.merge {
                    self.expression(ix, merge, owner);
                }
            }
            Pattern::Literal(_) | Pattern::Cut => {}
        }
    }

    fn touch(&mut self, field: &str, owner: Owner) {
        self.touches
            .entry(field.to_string())
            .or_default()
            .insert(owner);
    }

    fn block(&mut self, ix: &Index<'_>, block: &Block, owner: Owner) {
        for stmt in &block.stmts {
            if let Stmt::Assign { target, .. } = &stmt.node
                && let Some(name) = field_name(target)
            {
                self.touch(ix.text(name), owner);
            }
            visit_stmt(ix.parsed, &stmt.node, &mut |e| self.expr(ix, e, owner));
            visit_stmt_blocks(&stmt.node, &mut |b| self.block(ix, b, owner));
        }
    }

    fn expression(&mut self, ix: &Index<'_>, expr: &Expr, owner: Owner) {
        visit_expr(ix.parsed, expr, &mut |e| self.expr(ix, e, owner));
        visit_expr_blocks(expr, &mut |b| self.block(ix, b, owner));
    }

    fn expr(&mut self, ix: &Index<'_>, expr: &Expr, owner: Owner) {
        let names = self.names.entry(owner).or_default();
        let mut handed: &[Expr] = &[];
        let mut callees: Vec<usize> = Vec::new();
        match expr {
            Expr::MethodCall { method, args, .. } | Expr::SafeMethod { method, args, .. } => {
                names.insert(ix.text(*method).to_string());
                handed = args;
                callees = ix
                    .by_name
                    .get(ix.text(*method))
                    .cloned()
                    .unwrap_or_default();
            }
            Expr::Call { func, args, .. } => {
                handed = args;
                callees = ix.callees(func);
            }
            Expr::Variable(name, _) => {
                names.insert(ix.text(*name).to_string());
            }
            Expr::Path(segments, _) => {
                names.extend(segments.iter().map(|s| ix.text(*s).to_string()));
            }
            Expr::StructLit { name, .. } => {
                for (field, _) in ix.whole_fields(ix.text(*name)) {
                    self.touch(&field, owner);
                }
                // `Self { … }` builds the `impl`'s type, whichever it is.
                if ix.text(*name) == "Self" {
                    for fields in ix.structs.values() {
                        for (field, ty, _) in fields {
                            if range_of(&ty.name).is_some() {
                                self.touch(field, owner);
                            }
                        }
                    }
                }
            }
            Expr::With { fields, .. } => {
                for f in fields {
                    self.touch(ix.text(f.name), owner);
                }
            }
            Expr::Unary {
                op: UnaryOp::Ref,
                expr: inner,
                ..
            } => {
                if let Some(name) = field_name(inner) {
                    self.touch(ix.text(name), owner);
                }
            }
            // A lambda inside an f-string's hole, which no block walk reaches.
            Expr::Closure { body, .. } => self.block(ix, body, owner),
            // What a `spawn` runs is read as the code that holds it.
            Expr::Spawn { body, .. } => self.expression(ix, body, owner),
            _ => {}
        }
        for (i, arg) in handed.iter().enumerate() {
            let Some(name) = field_name(arg) else {
                continue;
            };
            let kept = !callees.is_empty()
                && callees
                    .iter()
                    .all(|at| match ix.fns.get(at).map(|f| &f.item.node) {
                        Some(Item::Fn { args, .. }) => args.get(i).is_some_and(|a| !a.mutable),
                        _ => false,
                    });
            if !kept {
                self.touch(ix.text(name), owner);
            }
        }
    }
}

/// `k * steps + k0 * pieces`, as a term.
fn bound_term(a: &mut TermArena, k: i64, k0: i64, steps: i64, pieces: i64) -> i64 {
    let k = a.int(k);
    let k0 = a.int(k0);
    let grown = a.mul(k, steps);
    let started = a.mul(k0, pieces);
    a.add(grown, started)
}

/// `|x| <= k * steps + k0 * pieces`.
fn invariant_term(a: &mut TermArena, k: i64, k0: i64, x: i64, steps: i64, pieces: i64) -> i64 {
    let bound = bound_term(a, k, k0, steps, pieces);
    let below = a.neg(bound);
    let low = a.le(below, x);
    let high = a.le(x, bound);
    a.and(vec![low, high])
}

fn proved(arena: &TermArena, facts: &[i64], goal: i64) -> bool {
    crate::proofs::ask(arena, facts, goal) == crate::proofs::Asked::Proved
}

/// **D3's invariant, checked before it is used** (ADR-361 D4): `|f| <= k ·
/// steps + k0 · pieces`, `steps` and `pieces` the ghost counts of steps and of
/// initial values. The initial value satisfies it, a step keeps it with
/// `steps + 1`, and a merge, as a sum or as one of the two, keeps it with
/// `steps_a + steps_b`.
fn invariant_checked(k: u128, k0: u128, merged: bool) -> bool {
    let (Ok(k), Ok(k0)) = (i64::try_from(k), i64::try_from(k0)) else {
        return false;
    };
    // The initial value.
    let mut a = TermArena::empty();
    let f = a.var("f");
    let (low, high) = (a.int(-k0), a.int(k0));
    let facts = vec![a.le(low, f), a.le(f, high)];
    let (none, one) = (a.int(0), a.int(1));
    let goal = invariant_term(&mut a, k, k0, f, none, one);
    if !proved(&a, &facts, goal) {
        return false;
    }
    // A step.
    let mut a = TermArena::empty();
    let (f, d, s, p) = (a.var("f"), a.var("d"), a.var("steps"), a.var("pieces"));
    let held = invariant_term(&mut a, k, k0, f, s, p);
    let (zero, low, high, one) = (a.int(0), a.int(-k), a.int(k), a.int(1));
    let facts = vec![
        held,
        a.ge(s, zero),
        a.ge(p, zero),
        a.le(low, d),
        a.le(d, high),
    ];
    let (after, next) = (a.add(f, d), a.add(s, one));
    let goal = invariant_term(&mut a, k, k0, after, next, p);
    if !proved(&a, &facts, goal) {
        return false;
    }
    if !merged {
        return true;
    }
    // A merge: the sum of both, or either one.
    for kept in [None, Some(0), Some(1)] {
        let mut a = TermArena::empty();
        let (fa, sa, pa) = (a.var("f_a"), a.var("steps_a"), a.var("pieces_a"));
        let (fb, sb, pb) = (a.var("f_b"), a.var("steps_b"), a.var("pieces_b"));
        let zero = a.int(0);
        let mut facts = vec![
            invariant_term(&mut a, k, k0, fa, sa, pa),
            invariant_term(&mut a, k, k0, fb, sb, pb),
        ];
        for ghost in [sa, sb, pa, pb] {
            facts.push(a.ge(ghost, zero));
        }
        let value = match kept {
            None => a.add(fa, fb),
            Some(0) => fa,
            _ => fb,
        };
        let (steps, pieces) = (a.add(sa, sb), a.add(pa, pb));
        let goal = invariant_term(&mut a, k, k0, value, steps, pieces);
        if !proved(&a, &facts, goal) {
            return false;
        }
    }
    true
}

/// The folds a pattern holds.
pub(crate) fn folds<'p>(pattern: &'p Pattern, out: &mut Vec<&'p FoldSpec>) {
    match pattern {
        Pattern::Fold(spec) => out.push(spec),
        Pattern::Seq(parts) | Pattern::Choice(parts) => {
            parts.iter().for_each(|p| folds(&p.node, out))
        }
        Pattern::Bind { pat, .. } | Pattern::Group(pat) | Pattern::Repeat { pat, .. } => {
            folds(&pat.node, out)
        }
        Pattern::Ref { args, .. } => args.iter().for_each(|p| folds(&p.node, out)),
        Pattern::Literal(_) | Pattern::Cut => {}
    }
}

/// **The counters of every fold over a grammar's input that ADR-367 D3
/// proves**, as the compound assignments whose check is not needed, by the
/// statement's first byte. `arithmetic` is the checker's table of the `+`,
/// `-` and `*` whose two sides are one whole-number type, with that type.
pub fn proven_counters(parsed: &Parsed, arithmetic: &BTreeMap<i64, String>) -> BTreeSet<i64> {
    let ix = Index::of(parsed);
    let mut out = BTreeSet::new();
    // A method of a whole number that changes it in place changes a field
    // where no write shows it.
    for item in &parsed.program.items {
        if let Item::Impl {
            target, methods, ..
        } = &item.node
            && range_of(&Ty::of(parsed, target).name).is_some()
            && methods.iter().any(|m| {
                matches!(&m.node, Item::Fn { receiver: Some(r), .. } if r.is_mut)
                    || matches!(&m.node, Item::Fn { args, .. } if args.iter().any(|a| a.mutable))
            })
        {
            return out;
        }
    }
    let uses = Uses::of(&ix);
    for item in &parsed.program.items {
        let Item::Grammar(grammar) = &item.node else {
            continue;
        };
        // ADR-367 D2: the step count starts from the input's extent. A
        // grammar over input that arrives while it parses gives
        // `Extent::Unbounded`, and nothing below is proved for it.
        let extent = input_extent(grammar);
        for rule in &grammar.rules {
            let mut specs = Vec::new();
            for alt in &rule.alts {
                folds(&alt.pattern.node, &mut specs);
            }
            for spec in specs {
                let at = rule.span.start as usize;
                out.extend(crate::proofs::in_function(
                    parsed.text(rule.name),
                    at,
                    || fold_counters(&ix, &uses, grammar, at, spec, extent, arithmetic),
                ));
            }
        }
    }
    out
}

/// D3 for one fold.
fn fold_counters(
    ix: &Index<'_>,
    uses: &Uses,
    grammar: &GrammarDef,
    rule_at: usize,
    spec: &FoldSpec,
    extent: Extent,
    arithmetic: &BTreeMap<i64, String>,
) -> Vec<i64> {
    let parsed = ix.parsed;
    let Some(width) = rule_width(parsed, grammar, parsed.text(spec.rule)) else {
        return Vec::new();
    };
    // No step count, no proof: an unbounded input (D2).
    let Some(steps) = most_steps(extent, width) else {
        return Vec::new();
    };
    // The accumulator's type, from what starts it.
    let acc = match &spec.init {
        Expr::Variable(name, _) => parsed.text(*name),
        Expr::Call { func, args, .. } if args.is_empty() => match &**func {
            Expr::Variable(name, _) => parsed.text(*name),
            _ => return Vec::new(),
        },
        Expr::StructLit { name, .. } => parsed.text(*name),
        _ => return Vec::new(),
    };
    if !ix.structs.contains_key(acc) {
        return Vec::new();
    }
    let acc = Ty::named(acc);
    // **The initial value counts as a step's** (D3).
    let mut init = Walk::new(ix);
    match &spec.init {
        Expr::Variable(..) => {
            let starts: Vec<usize> = ix
                .methods
                .get(&(acc.name.clone(), String::new()))
                .into_iter()
                .flatten()
                .copied()
                .filter(|at| {
                    matches!(ix.fns.get(at).map(|f| &f.item.node),
                        Some(Item::Fn { args, .. }) if args.is_empty())
                })
                .collect();
            if starts.is_empty() {
                return Vec::new();
            }
            for at in starts {
                init.function(at, false);
            }
        }
        other => init.expression(other, &Env::new(), false),
    }
    // **The step**, through its lambda and everything it calls.
    let mut step = Walk::new(ix);
    match &spec.step {
        Expr::Closure { params, body, .. } if params.len() == 2 => {
            let mut env = Env::new();
            env.insert(parsed.text(params[0]).to_string(), acc.clone());
            step.block(body, &env, false);
        }
        Expr::Path(..) => {
            let callees = ix.callees(&spec.step);
            if callees.is_empty() {
                return Vec::new();
            }
            for at in callees {
                step.function(at, false);
            }
        }
        _ => return Vec::new(),
    }
    // **The merge**: a sum of both partial results, or one of them.
    let mut merge = Merge {
        ix,
        sites: BTreeMap::new(),
        bad: BTreeSet::new(),
        reached: BTreeSet::new(),
        broken: false,
        stack: Vec::new(),
    };
    if let Some(merging) = &spec.merge {
        let callees = match merging {
            Expr::Path(..) => ix.callees(merging),
            Expr::Closure { params, body, .. } if params.len() == 2 => {
                match body.stmts.as_slice() {
                    [
                        Spanned {
                            node:
                                Stmt::Expr(Expr::MethodCall {
                                    receiver,
                                    method,
                                    args,
                                    ..
                                }),
                            ..
                        },
                    ] if matches!(&**receiver, Expr::Variable(v, _) if *v == params[0])
                        && (matches!(args.as_slice(),
                        [Expr::Variable(v, _)] if *v == params[1])
                            || matches!(args.as_slice(), [Expr::Unary { op: UnaryOp::Ref, expr, .. }]
                            if matches!(&**expr, Expr::Variable(v, _) if *v == params[1]))) =>
                    {
                        ix.methods_of(Some(&acc), parsed.text(*method))
                    }
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        };
        let [callee] = callees.as_slice() else {
            return Vec::new();
        };
        merge.function(*callee);
    }
    if init.broken || step.broken || merge.broken {
        return Vec::new();
    }
    // **Where the proof applies: only inside the fold** (D3). A function of
    // the fold that code outside it can reach runs where no step count holds.
    let region: BTreeSet<usize> = init
        .reached
        .iter()
        .chain(&step.reached)
        .chain(&merge.reached)
        .copied()
        .collect();
    let me = Owner::Fold(rule_at);
    let inside = |owner: &Owner| match owner {
        Owner::Function(at) => region.contains(at),
        other => *other == me,
    };
    let mut outside: BTreeSet<usize> = region.intersection(&ix.implicit).copied().collect();
    let mut queue: Vec<&String> = outside
        .iter()
        .filter_map(|at| uses.names.get(&Owner::Function(*at)))
        .flatten()
        .chain(
            uses.names
                .iter()
                .filter(|(owner, _)| !inside(owner))
                .flat_map(|(_, names)| names.iter()),
        )
        .collect();
    let mut seen: BTreeSet<&String> = BTreeSet::new();
    while let Some(name) = queue.pop() {
        if !seen.insert(name) {
            continue;
        }
        for at in ix.reached_by.get(name).into_iter().flatten() {
            if region.contains(at) && outside.insert(*at) {
                queue.extend(uses.names.get(&Owner::Function(*at)).into_iter().flatten());
            }
        }
    }
    let only_inside = |owner: &Owner| match owner {
        Owner::Function(at) => region.contains(at) && !outside.contains(at),
        other => *other == me,
    };
    let fields: BTreeSet<&String> = step
        .sites
        .keys()
        .chain(merge.sites.keys())
        .chain(init.sites.keys())
        .collect();
    let mut out = Vec::new();
    for field in fields {
        let (Some(k), Some(k0)) = (
            step.k.get(field).copied().unwrap_or(Some(0)),
            init.k.get(field).copied().unwrap_or(Some(0)),
        ) else {
            continue;
        };
        // In a `par_fold` each piece starts from an initial value, and the
        // pieces are not counted: only a field that starts at nothing.
        if spec.parallel && k0 > 0 {
            continue;
        }
        if merge.bad.contains(field) {
            continue;
        }
        if !uses
            .touches
            .get(field)
            .is_none_or(|owners| owners.iter().all(only_inside))
        {
            continue;
        }
        if !invariant_checked(k, k0, spec.merge.is_some()) {
            continue;
        }
        let lowered = step.lowered.contains(field) || init.lowered.contains(field);
        let pieces = u128::from(!spec.parallel);
        let Some(bound) = k
            .checked_mul(steps)
            .and_then(|b| b.checked_add(k0 * pieces))
            .and_then(|b| i128::try_from(b).ok())
        else {
            continue;
        };
        let sites = [&step.sites, &merge.sites, &init.sites]
            .into_iter()
            .filter_map(|s| s.get(field))
            .flatten();
        for site in sites {
            let Some((low, high)) = arithmetic.get(site).and_then(|ty| range_of(ty)) else {
                continue;
            };
            let fits = bound <= high
                && match low {
                    0 => !lowered,
                    _ => -bound >= low,
                };
            if fits {
                out.push(*site);
            }
        }
    }
    out
}
