fn home() -> Option<String> {
    let h = std::env::var("HOME").ok()?;
    Some(h)
}

fn port() -> Option<u16> {
    std::env::var("PORT").ok().and_then(|p| p.parse().ok())
}

fn first(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("a")).ok()?.trim().to_string();
    Some(text)
}

fn manifest(dir: &Path) -> Option<Manifest> {
    let m = Manifest::read(dir).ok()?;
    Some(m)
}

fn ledger(text: &str) -> Option<Ledger> {
    Ledger::parse(text).ok()
}

fn ctx(path: &Path) -> Result<String> {
    let t = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(t)
}

fn flag() -> bool {
    std::env::var("X").ok().map(|v| v == "1").unwrap_or(false)
}

fn ctx2(path: &Path) -> Result<usize> {
    let n = std::fs::read_to_string(path).context("reading")?.len();
    let m = std::fs::read(path).with_context(|| "bytes")?;
    Ok(n + m.len())
}
