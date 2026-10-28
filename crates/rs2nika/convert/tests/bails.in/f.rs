fn named(e: Expr) -> Result<String> {
    match e {
        Expr::Variable(name) => Ok(name),
        Expr::Call(f) => anyhow::bail!("`{f}` is a call"),
        _ => anyhow::bail!("not a name"),
    }
}
