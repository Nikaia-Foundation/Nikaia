// crates/nikaia/src/bounds_report.rs
//
// `--bounds`: every index into a list and every `+`, `-` and `*` the
// optimizations of ADR-306 may drop the check of, and what became of each
// (#389).
//
// An author could see what `remove-bounds-checks` and `remove-overflow-checks`
// proved only by reading the lowered Rust for `proven::read` and
// `<T>::wrapping_*`. `--asserts` already answers the same question for written
// claims (ADR-269 D7); this is that report for the checks the compiler writes
// itself. It explains a decision and changes none, as `--trust` and `--tethers`
// do.
//
// **The reason a check stayed is the shape that stopped the walk, read off the
// site**, not a trace of the solver: a field, a list of lists, a product of two
// unknowns, a call's result, or none of those - a value the walk has no bound
// for, which is a counter, a loop or a length. Those are `solver-workload.md`
// §8.4's categories, each with the issue that would supply the fact. It names
// where the next fact pays off; it does not prove that the fact would be
// enough.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::ast::{BinaryOp, Block, Expr, Item, Span, Stmt};
use crate::bounds::{BoundsChecks, OverflowChecks};
use crate::check::value_node;
use crate::contracts::Ledger;
use crate::contracts::sync::{visit_stmt, visit_stmt_blocks};
use crate::parser::Parsed;

/// One site a check stands at.
struct Site {
    line: usize,
    /// The site as written, near enough (`check::written`).
    text: String,
    index: bool,
    proved: bool,
    /// Why it was not, where it was not.
    blocked: Option<String>,
}

/// The report, for one file. What is proved does not depend on what the
/// build drops (ADR-306 D14), only on `through_loops`
/// (`prove-through-loops:on`, ADR-361 D10); the first line says whether this
/// build writes it without its check.
#[allow(clippy::too_many_arguments)]
pub fn report(
    parsed: &Parsed,
    source: &str,
    path: &str,
    own: &Ledger,
    library: &Ledger,
    bounds: BoundsChecks,
    overflow: OverflowChecks,
    through_loops: bool,
) -> String {
    let checked = crate::check::check(parsed, own, library);
    let proven = crate::bounds::proven(
        parsed,
        &crate::bounds::Sites::of(&checked.facts),
        &[own, library],
        &std::collections::BTreeSet::new(),
        through_loops,
    );
    let mut walk = Walk {
        parsed,
        source,
        lists: &checked.facts.list_indices,
        arithmetic: &checked.facts.arithmetic,
        negations: &checked.facts.negations,
        proven: &proven,
        seen: HashSet::new(),
        sites: Vec::new(),
    };
    for item in &parsed.program.items {
        match &item.node {
            Item::Fn { body, .. } => walk.block(body),
            Item::Impl { methods, .. } => {
                for method in methods {
                    if let Item::Fn { body, .. } = &method.node {
                        walk.block(body);
                    }
                }
            }
            // A grammar's actions are walked too (ADR-314 D1).
            Item::Grammar(grammar) => {
                for rule in &grammar.rules {
                    for alt in &rule.alts {
                        if let Some(action) = &alt.action {
                            walk.block(action);
                        }
                        // And a fold's lambdas (ADR-367 D3).
                        let mut specs = Vec::new();
                        crate::fold_bound::folds(&alt.pattern.node, &mut specs);
                        for spec in specs {
                            for lambda in [Some(&spec.init), Some(&spec.step), spec.merge.as_ref()]
                                .into_iter()
                                .flatten()
                            {
                                if let crate::ast::Expr::Closure { body, .. } = lambda {
                                    walk.block(body);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let mut sites = walk.sites;
    sites.sort_by_key(|s| s.line);
    let indexes = sites.iter().filter(|s| s.index).count();
    let operations = sites.len() - indexes;
    let proved = |index: bool| {
        sites
            .iter()
            .filter(|s| s.index == index && s.proved)
            .count()
    };
    let mut out = format!(
        "bounds in {path}: {} indexes, {} proved; {} operations, {} proved - written without \
         their check: indexes {}, operations {}\n",
        indexes,
        proved(true),
        operations,
        proved(false),
        bounds.name(),
        overflow.name(),
    );
    let width = sites
        .iter()
        .map(|s| s.text.len())
        .max()
        .unwrap_or(0)
        .min(40);
    for site in &sites {
        let how = match (site.proved, &site.blocked) {
            (true, _) => "proved".to_string(),
            (false, Some(why)) => format!("checked: {why}"),
            (false, None) => "checked".to_string(),
        };
        out.push_str(&format!(
            "  {:>4}  {:<width$}  {how}\n",
            site.line,
            site.text,
            width = width
        ));
    }
    out
}

struct Walk<'a> {
    parsed: &'a Parsed,
    source: &'a str,
    lists: &'a BTreeSet<i64>,
    arithmetic: &'a BTreeMap<i64, String>,
    negations: &'a BTreeMap<i64, String>,
    proven: &'a crate::bounds::Proven,
    /// Index nodes already listed: an index read in a statement is listed once.
    seen: HashSet<usize>,
    sites: Vec<Site>,
}

impl Walk<'_> {
    fn line(&self, at: usize) -> usize {
        self.source[..at.min(self.source.len())]
            .matches('\n')
            .count()
            + 1
    }

    fn block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.stmt(&stmt.node, &stmt.span);
            visit_stmt_blocks(&stmt.node, &mut |inner| self.block(inner));
        }
    }

    fn stmt(&mut self, stmt: &Stmt, span: &Span) {
        let line = self.line(span.at());
        // **A compound assignment is its operation** (ADR-306 D5a): keyed by
        // the statement's first byte, where no operator of an expression stands.
        if let Stmt::Assign {
            target,
            op: Some(op @ (BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul)),
            value,
        } = stmt
            && self.arithmetic.contains_key(&(span.at() as i64))
        {
            let proved = self.proven.arithmetic.contains(&(span.at() as i64));
            let text = format!(
                "{} {}= {}",
                crate::check::written(self.parsed, target),
                symbol(*op),
                crate::check::written(self.parsed, value)
            );
            self.sites.push(Site {
                line,
                text,
                index: false,
                proved,
                blocked: (!proved).then(|| blocked_operation(*op, target, value)),
            });
        }
        let mut found: Vec<Site> = Vec::new();
        visit_stmt(self.parsed, stmt, &mut |expr| match expr {
            Expr::Index { base, index, .. } if self.lists.contains(&value_node(base)) => {
                if !self.seen.insert(value_node(base) as usize) {
                    return;
                }
                let proved = self.proven.indices.contains(&value_node(base));
                found.push(Site {
                    line,
                    text: crate::check::written(self.parsed, expr),
                    index: true,
                    proved,
                    blocked: (!proved).then(|| blocked_index(base, index)),
                });
            }
            // **A word read of a `Bytes`** (ADR-320 D12), a candidate as the
            // cut it reads is: keyed by its receiver.
            Expr::MethodCall { receiver, args, .. }
                if args.len() == 1 && self.lists.contains(&value_node(receiver)) =>
            {
                if !self.seen.insert(value_node(receiver) as usize) {
                    return;
                }
                let proved = self.proven.indices.contains(&value_node(receiver));
                found.push(Site {
                    line,
                    text: crate::check::written(self.parsed, expr),
                    index: true,
                    proved,
                    blocked: (!proved).then(|| blocked_index(receiver, &args[0])),
                });
            }
            Expr::Binary {
                op: op @ (BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul),
                lhs,
                rhs,
                span,
                ..
            } if self.arithmetic.contains_key(&(span.at() as i64)) => {
                let proved = self.proven.arithmetic.contains(&(span.at() as i64));
                found.push(Site {
                    line: self.line(span.at()),
                    text: crate::check::written(self.parsed, expr),
                    index: false,
                    proved,
                    blocked: (!proved).then(|| blocked_operation(*op, lhs, rhs)),
                });
            }
            // **A negation** (ADR-314 D5): it leaves its type only from the
            // type's least value.
            Expr::Unary {
                op: crate::ast::UnaryOp::Neg,
                ..
            } if self.negations.contains_key(&value_node(expr)) => {
                let proved = self.proven.negations.contains(&value_node(expr));
                found.push(Site {
                    line,
                    text: crate::check::written(self.parsed, expr),
                    index: false,
                    proved,
                    blocked: (!proved)
                        .then(|| "no fact keeps it above its type's least value".to_string()),
                });
            }
            _ => {}
        });
        self.sites.extend(found);
    }
}

fn symbol(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        _ => "*",
    }
}

/// **Why a check stayed** is `tools/bounds_reasons.nika` (ADR-294, #435).
fn blocked_index(base: &Expr, index: &Expr) -> String {
    nikaia_std::tools::bounds_reasons::blocked_index(base, index)
}

fn blocked_operation(op: BinaryOp, lhs: &Expr, rhs: &Expr) -> String {
    nikaia_std::tools::bounds_reasons::blocked_operation(&op, lhs, rhs)
}
