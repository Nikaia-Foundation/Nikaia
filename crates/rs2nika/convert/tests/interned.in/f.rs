fn rename(items: &mut Vec<Name>, interner: &Interner) -> Result<Symbol> {
    let main = match parser::parse_expression(interner, "main")? {
        crate::ast::Expr::Variable(name, _) => name,
        _ => anyhow::bail!("`main` did not parse as a name"),
    };
    let other = interner.intern_string("other");
    Ok(main)
}
