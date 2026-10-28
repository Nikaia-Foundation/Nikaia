fn a1(path: &Path) -> Result<PathBuf> {
    let p = path.canonicalize()?;
    let q = std::fs::canonicalize(&p)?;
    Ok(q)
}

fn a2(path: &Path, root: &Path) -> Option<PathBuf> {
    let r = path.strip_prefix(root).ok()?;
    Some(r.to_path_buf())
}

fn a3(path: &Path, root: &Path) -> String {
    let rest = path.strip_prefix(root).unwrap_or(path);
    rest.display().to_string()
}

fn a4(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// A test's file as the project names it.
fn shown_name(path: &Path, root: &Path) -> String {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    path.strip_prefix(&root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}
