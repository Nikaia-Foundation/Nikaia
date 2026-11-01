struct Doc {
    body: String,
    marks: Vec<u8>,
}

fn tail_of(text: &str, at: usize) -> Result<String, String> {
    let rest = &text[at..];
    Ok(rest.to_string())
}

fn head_of(text: &str, at: usize) -> String {
    text[..at].to_string()
}

fn middle(text: &String, a: usize, b: usize) -> String {
    let cut = &text[a..b];
    cut.to_string()
}

fn opens(text: &str, at: usize) -> bool {
    text[at..].starts_with("impl") || text[..at].ends_with('}')
}

fn rest_from(text: &str, at: usize) -> String {
    let rest = text.get(at..).unwrap_or_default();
    rest.to_string()
}

fn first_word(text: &str) -> String {
    text.split(' ').next().unwrap_or_default().to_string()
}

fn whole(text: &str) -> String {
    text[..].to_string()
}

fn field_cut(doc: &Doc, at: usize) -> String {
    doc.body[at..].to_string()
}

fn of_bytes(text: &str, at: usize) -> usize {
    let bytes = text.as_bytes();
    bytes[at..].len() + doc_marks(&bytes[..at])
}

fn unknown(xs: Thing, at: usize) -> usize {
    xs[at..].len()
}

fn found(text: &str, at: usize) -> Option<usize> {
    let end = text[at + 1..].find('"')?;
    Some(end)
}

fn copy_cut(text: &str, from: usize, to: usize, out: &mut String) {
    out.push_str(&text[from..to]);
}
