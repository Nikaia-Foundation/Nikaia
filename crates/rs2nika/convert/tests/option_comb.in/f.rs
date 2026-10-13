fn tail_of(block: &Block) -> Option<&Expr> {
    block.stmts.last().map(|s| &s.node)
}

fn named(t: &Option<Ty>) -> bool {
    t.as_ref().is_some_and(|t| t.is_named())
}

fn width(t: Option<&Ty>) -> i64 {
    t.map_or(0, |t| t.name.len())
}

fn sizes(xs: &[Ty]) -> Vec<i64> {
    xs.iter().map(|x| x.size).collect()
}
