fn is_text(e: &Expr) -> bool {
    match e {
        Expr::LitStr { .. } | Expr::LitInterpolated { .. } => true,
        _ => false,
    }
}

fn error_of(t: &Ty) -> Option<&Ty> {
    match t {
        Ty::Named { name, args, view: false } if name == "Error" => args.first(),
        _ => None,
    }
}
