fn is_call(e: &Expr) -> bool {
    match e {
        Expr::Call(..) => true,
        Expr::Index(base, ..) => is_call(base),
        Expr::Pair(_, rhs, ..) if rhs.is_empty() => false,
        _ => false,
    }
}

enum Shape {
    Pair(Expr, Expr, Span),
    Index(Expr, Expr),
}

fn first_of(s: &Shape) -> bool {
    match s {
        Shape::Pair(a, ..) => is_call(a),
        Shape::Index(_, i, ..) => is_call(i),
    }
}
