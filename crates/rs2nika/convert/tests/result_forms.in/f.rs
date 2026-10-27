fn width(name: Option<&str>) -> usize {
    name.unwrap_or("").len()
}

fn first(a: Option<i64>, b: Option<i64>) -> i64 {
    let x = a.or_else(|| b).unwrap_or(0);
    x
}

fn read(path: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let n = text.trim().parse::<usize>().map_err(|_| "not a number".to_string())?;
    if n == 0 {
        return Err("zero".to_string());
    }
    Ok(text)
}

fn count(path: &str) -> usize {
    read(path).unwrap_or_else(|_| String::new()).len()
}

fn need(v: Option<i64>) -> Result<i64, String> {
    let x = v.ok_or("missing".to_string())?;
    Ok(x + 1)
}

fn head(path: &str) -> Vec<String> {
    let Ok(text) = read(path) else {
        return Vec::new();
    };
    vec![text]
}

fn digits(s: &str) -> Option<i64> {
    let n = s.parse::<i64>().ok()?;
    Some(n)
}
