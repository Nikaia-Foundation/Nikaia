fn kind(func: &Box<Expr>) -> u8 {
    match *func {
        Expr::Variable(..) => 1,
        _ => 0,
    }
}

fn jobs(text: &str) -> Result<u32, Refused> {
    if text.is_empty() {
        refused!("`--jobs` takes a number, not `{text}`");
    }
    if text == "-" {
        refused!("`--jobs` takes no dash");
    }
    Ok(0)
}
