fn listed(dir: &Path) -> Result<Vec<String>> {
    let optional = |path: PathBuf| -> Result<Option<Vec<u8>>> {
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
        }
    };
    let mut out = Vec::new();
    if dir.is_dir() && optional(dir.join("a"))?.is_some() {
        out.push("a".to_string());
    }
    Ok(out)
}

fn other(dir: &Path) -> Result<bool> {
    let optional = |path: PathBuf| -> Result<Option<Vec<u8>>> {
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
        }
    };
    Ok(dir.join("b").is_file() && dir.exists() && optional(dir.join("b"))?.is_none())
}
