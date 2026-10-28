fn method_of(e: &Expr) -> String {
    match e {
        Expr::MethodCall { method, .. } | Expr::SafeMethod { method, .. } => {
            method.clone()
        }
        Expr::Call { .. } | Expr::Index { .. } => String::new(),
        _ => String::new(),
    }
}
