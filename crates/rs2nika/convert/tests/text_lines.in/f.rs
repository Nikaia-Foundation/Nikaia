/// The pieces of a text.
fn pieces(text: &str) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for line in text.split_inclusive('\n') {
        match line.strip_prefix("# ") {
            Some(name) => out.push(name.to_string()),
            None => out.last_mut()?.push_str(line),
        }
    }
    (!out.is_empty()).then_some(out)
}
