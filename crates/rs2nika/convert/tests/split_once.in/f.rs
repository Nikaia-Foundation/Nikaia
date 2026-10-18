fn cut(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim().to_string(), value.to_string()))
}
fn owner(path: &str) -> (&str, &str) {
    path.rsplit_once("::").unwrap_or(("", path))
}
fn none_yet() -> Option<(String, i64)> {
    let found: Option<(String, i64)> = None;
    found
}
