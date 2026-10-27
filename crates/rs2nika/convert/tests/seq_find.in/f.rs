fn seq_probe(outer: &[View], root: &str, detail: &str) -> Option<usize> {
    let held = outer.iter().rev().find(|v| v.name == root)?;
    let line = detail.lines().find(|line| line.contains("stopped")).unwrap_or(detail);
    let cut = root.find(|c: char| c.is_ascii_digit()).unwrap_or(0);
    let short = root.trim_start_matches(|c: char| c.is_ascii_digit());
    let shouted = root.to_uppercase().replace(['-', '.'], "_");
    Some(held.at + line.len() + cut + short.len() + shouted.len())
}
