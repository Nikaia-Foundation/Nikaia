pub fn changed_lines(a: &str, b: &str) -> Vec<String> {
    nikaia_std::tools::diffs::changed_lines(a, b)
}

pub fn w(a: &str, b: &str) -> Vec<String> {
    let mut lines = changed_lines(a, b);
    lines.push(crate::proofs::FILE.to_string());
    lines
}
