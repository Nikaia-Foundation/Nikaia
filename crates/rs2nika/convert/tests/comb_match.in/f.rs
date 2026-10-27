fn mine(key: &str, own: &BTreeMap<String, i64>) -> bool {
    !key.split_once("::").is_some_and(|(package, _)| own.contains_key(package))
}
fn first_ok(name: &str) -> bool {
    name.strip_prefix("__t_").is_some_and(|k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()))
}
fn at(source: &str, at: usize) -> usize {
    source.char_indices().nth(at).map_or(source.len(), |(byte, _)| byte)
}
fn committed(root: &Path) -> Option<Ledger> {
    std::fs::read_to_string(root.join("c")).ok().and_then(|text| Ledger::parse(&text).ok())
}
fn stays(r: Result<i64, String>) -> Result<i64, String> {
    r.map(|n| n + 1)
}
