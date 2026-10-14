fn an_or_a(ty: &str) -> &str {
    if ty.starts_with(['i', 'a']) { "an" } else { "a" }
}

fn wide(o: Option<String>) -> bool {
    o.is_some_and(|x| x.len() > 1 && x.len() < 5)
}

fn head(v: &[&str]) -> Option<usize> {
    v.iter().find_map(|s| { let t = s.strip_prefix("a")?; Some(t.len()) })
}
