fn note(e: &Expr, names: &mut BTreeSet<String>) {
    if let Expr::Lit(parts) = e {
        for part in parts {
            let mut inner = BTreeSet::new();
            crate::emit::visit_expr(part, &mut |e| note(e, &mut inner));
            names.extend(inner);
        }
    }
}
