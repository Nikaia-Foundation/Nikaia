use std::path::Path;

fn literal_of(part: &str) -> Option<i64> {
    part.parse().ok()
}

pub(crate) fn literals(parts: &[String]) -> Option<Vec<i64>> {
    let args = parts.iter().map(|part| literal_of(part)).collect::<Option<Vec<_>>>()?;
    Some(args)
}

pub(crate) fn stem(path: &Path) -> bool {
    path.file_stem().is_some_and(|s| s == "main")
}

pub(crate) fn either(a: Option<i64>, b: Option<i64>) -> Option<i64> {
    a.or_else(|| b)
}

pub(crate) fn owned(path: &Path) -> std::path::PathBuf {
    path.to_path_buf()
}

pub(crate) fn numbered(xs: &[String]) -> Vec<(usize, String)> {
    xs.iter().cloned().enumerate().collect()
}
