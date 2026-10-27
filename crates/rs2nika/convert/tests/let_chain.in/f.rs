fn header_of(code: i64, header: Option<String>) -> i64 {
    if code == 0 && let Some(header) = header {
        return header.len() as i64;
    }
    0
}
fn flags(a_program: bool) -> usize {
    if a_program && let Some(flags) = std::env::var_os(LINK_VAR) && !flags.is_empty() {
        return flags.len();
    }
    0
}
fn peeked(line: &str, next: Option<&str>) -> bool {
    if line == "x" && let Some(n) = next && n.starts_with("use ") {
        return true;
    }
    false
}
