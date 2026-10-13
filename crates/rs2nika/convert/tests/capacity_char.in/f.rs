fn quoted(text: &str, byte: u8) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    out.push(byte as char);
    out
}

fn all_known(xs: &[Ty]) -> bool {
    xs.iter().all(wholly_known) && xs.iter().any(|x| x.is_named())
}
