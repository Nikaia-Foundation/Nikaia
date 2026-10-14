fn listed(shown: &[String], n: usize) -> String {
    match n {
        0 => String::new(),
        _ => format!("{} and `{}`", shown.join(", "), shown.len()),
    }
}

fn pick(o: Option<i64>, strict: bool) -> i64 {
    match o {
        Some(x) if x > 0 => x,
        None if strict => return -1,
        Some(_) => 1,
        None => 0,
    }
}
