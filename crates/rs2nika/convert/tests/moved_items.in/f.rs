struct Outcome {
    passed: bool,
    stdout: Vec<u8>,
    files: Vec<(String, Option<Vec<u8>>)>,
}

fn read_all(path: &Path) -> Result<Vec<u8>> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(bytes)
}

fn helper(n: i64) -> i64 {
    n + 1
}

fn entry(n: i64) -> i64 {
    helper(n) * 2
}
