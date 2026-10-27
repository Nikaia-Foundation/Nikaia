//! R188-R204: module lines, foreign `use`s, adapters, and patterns.
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

fn hashed() -> u64 {
    let basis = 0xcbf2_9ce4_8422_2325u64;
    basis ^ 0b1010u8 as u64 ^ 0xFFusize as u64
}

fn either(args: &[Arg], ret: Option<&Ty>) -> bool {
    if args.iter().map(|a| &a.ty).chain(ret.iter().copied()).any(|t| t.is_view) {
        return true;
    }
    let all = args.iter().map(|a| &a.ty).chain(ret.iter().copied()).all(|t| t.lasting) && args.len() > 1;
    all
}

fn gathered(first: &Unit, beside: &[Unit], keys: &BTreeMap<String, Ty>, more: &BTreeMap<String, Ty>) -> Vec<String> {
    let mut seen = 0;
    for unit in std::iter::once(first).chain(beside.iter().copied()) {
        seen += unit.len();
    }
    let names: Vec<String> = keys.keys().chain(more.keys()).cloned().collect();
    let all: Vec<&Unit> = beside.iter().chain(beside.iter()).collect();
    names
}

fn named(unit: &Parsed, word: Word) -> bool {
    unit.text(word) == "x"
}

impl Mark {
    fn text(&self) -> String {
        self.places.join(", ")
    }

    fn read(text: &str) -> Mark {
        Mark { places: vec![text.to_string()] }
    }
}

fn after_mark() -> usize {
    0
}

fn listed(places: &str, c: char) -> String {
    let q = if c == '"' { 1 } else { 2 };
    format!("{places}; repeatable {q}")
}

#[derive(Debug, Clone, Default)]
pub struct Moved {
    /// The names.
    pub names: Vec<String>,
    pub by: BTreeMap<String, usize>,
    pub last: Option<String>,
    pub done: bool,
}

impl Moved {
    pub fn new() -> Moved {
        Moved::default()
    }
}

fn fresh() -> Moved {
    Moved::new()
}

impl Checker<'_> {
    fn spelled(
        &mut self,
        value: &Expr,
    ) -> String {
        let here = self.parsed;
        self.written(here, value)
    }

    fn twice(&mut self, value: &Expr) -> String {
        self.spelled(value)
    }
}

impl Checker<'_> {
    fn walked(&mut self) {
        let parsed = self.parsed;
        for item in &parsed.program.items {
            self.twice(&item.value);
        }
    }
}

fn told(with: &str, throw: usize) {
    println!("told {} with {throw} items", with);
    eprintln!();
    eprint!("no holes, with care");
    print!("{with}: '{}'", 'x');
}

fn lazy(xs: &[usize]) -> usize {
    let g = move || {
        let y = 1;
        y + 2
    };
    run(|| xs.len() + 1);
    g()
}
