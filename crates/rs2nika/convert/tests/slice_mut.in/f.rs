fn quiet(findings: &mut [check::Finding], level: &[u8; 4]) {
    findings.sort();
}
fn read(items: &[Item], fixed: [u8; 4]) -> usize {
    items.len()
}
