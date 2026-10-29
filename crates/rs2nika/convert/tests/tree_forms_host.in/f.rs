fn read_unit(source: &str, m: &Manifest) -> Result<usize> {
    let parsed = parser::parse_to_ast(source)?;
    let s = m.setting("x");
    Ok(parsed + s)
}
fn outer(source: &str, m: &Manifest) -> Result<usize> {
    read_unit(source, m)
}
