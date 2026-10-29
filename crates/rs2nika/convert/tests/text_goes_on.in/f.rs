fn a(d: &D) -> Result<()> {
    return Err(refuse(format!(
        "`{}` is by path.\n\
         A package with a `{SRC}/` folder.",
        d.name
    )));
}
fn b() -> String { format!("x {} with y", 1) }
fn c() -> String { "a `b` with c".to_string() }
