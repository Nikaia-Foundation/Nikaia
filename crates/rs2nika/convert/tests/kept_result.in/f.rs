fn load(path: &Path) -> Option<String> {
    let read = std::fs::read_to_string(path);
    match read {
        Ok(text) => Some(text),
        Err(e) => {
            eprintln!("cannot read {}: {}", path.display(), e);
            None
        }
    }
}

fn parse(s: &str) -> i64 {
    let r = s.parse::<i64>();
    match r {
        Ok(n) if n > 0 => n,
        Ok(_) => 0,
        Err(_) => -1,
    }
}

fn direct(s: &str) -> i64 {
    match s.parse::<i64>() {
        Ok(n) => n,
        Err(err) => {
            eprintln!("{err}");
            0
        }
    }
}

fn reused(s: &str) -> i64 {
    let r = s.parse::<i64>();
    let shown = format!("{:?}", r);
    match r {
        Ok(n) => n,
        Err(_) => shown.len() as i64,
    }
}

fn later(s: &str) -> i64 {
    let r = s.parse::<i64>();
    match r {
        Ok(n) => n,
        Err(_) => 0,
    };
    r.is_ok() as i64
}
