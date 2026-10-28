fn count(e: &Expr, n: &mut usize) {
    match e {
        Expr::Variable(_) => {
            *n += 1;
        }
        Expr::Call(_) => {
            *n += 2;
        }
        _ => {}
    }
}
