/// The names of the files.
fn names(files: &[(String, String)]) -> Vec<String> {
    files.iter().map(|(name, text)| format!("{name}: {}", text.len())).collect()
}
