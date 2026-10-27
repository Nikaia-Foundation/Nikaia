fn sources(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    paths.sort();
    Ok(paths)
}

fn stale(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

fn walk(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path);
        }
    }
}

fn count(dir: &Path) -> usize {
    std::fs::read_dir(dir).map(|entries| entries.flatten().count()).unwrap_or(0)
}

fn direct(dir: &Path) {
    for entry in std::fs::read_dir(dir).into_iter().flatten() {
        let path = entry.path();
        println!("{}", path.display());
    }
}
