/// Reads what the cache holds.
fn cached(path: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let _ = std::fs::metadata(path).ok()?;
    let _ = std::fs::remove_file(path);
    let ext = path.extension()?.to_str()?;
    let level = match ext {
        "nika" => {
            let n = text.len();
            n
        }
        _ => 0,
    };
    Some(format!("{text}{level}"))
}
