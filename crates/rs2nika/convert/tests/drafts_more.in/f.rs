//! R188-R203: module lines, foreign `use`s, adapters, and patterns.
mod inner;
pub(crate) mod helpers;
use std::collections::{BTreeMap, BTreeSet};
use crate::check::Finding;
use nikaia_std::tools::check_walk;

static CACHE: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());

impl std::fmt::Display for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "shape")
    }
}

impl Empty {}

trait Named {
    fn name(&self) -> String;
}

fn chosen(names: &[String], body: &Block, expected: Option<Ty>, tail: Option<Span>, how: Option<Wrap>) -> Vec<String> {
    let all: Vec<_> = names.iter().map(String::from).collect();
    let n = check_walk::count(body);
    let _ = crate::check::helpers::first(body);
    if expected.is_some() && let Some(Stmt::Expr(value)) = body.stmts.last().map(|s| &s.node) && let Some(at) = tail {
        use_both(value, at);
    }
    if let (Some(expected), Some(Stmt::Expr(value))) =
        (&expected, body.stmts.last().map(|s| &s.node))
    {
        use_both(value, expected);
    }
    match how {
        Some(how @ (Wrap::Widen(_) | Wrap::Constructor)) => {
            keep(how);
        }
        _ => {}
    }
    let typed: Vec<Ty> = names
        .iter()
        .filter_map(|n| Some(Ty::from(n.get(0..1)?)))
        // a comment between two links
        .collect();
    crate::contracts::sync::visit_stmt_blocks(&body.stmts[0].node, &mut |inner| {
        walk(inner)
    });
    all
}

fn evens(xs: &[i64]) -> impl Iterator<Item = i64> + '_ {
    xs.iter().copied().filter(|x| x % 2 == 0)
}
