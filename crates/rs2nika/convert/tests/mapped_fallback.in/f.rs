fn width(name: Option<&str>, other: &str) -> usize {
    name.map(|n| n.len()).unwrap_or_else(|| other.len())
}

fn label(name: Option<String>, other: &str) -> String {
    let shown = name.map(|n| format!("<{}>", n)).unwrap_or(format!("<{}>", other));
    shown
}

fn count(name: Option<&str>) -> usize {
    name.map(|n| n.len()).unwrap_or(0)
}
