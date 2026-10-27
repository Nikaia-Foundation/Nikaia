fn shown(file: Option<String>, function: Option<String>) -> String {
    match (file, function) {
        (Some(file), Some(function)) => format!("{file}, {function}"),
        (Some(file), None) => file,
        (None, Some(function)) => function,
        (None, None) => "nowhere".to_string(),
    }
}

fn flags(encoded: Option<String>, plain: Option<String>) -> Vec<String> {
    let flags: Vec<String> = match (encoded, plain) {
        // the encoded ones first
        (Some(e), _) => e.split(',').map(String::from).collect(),
        (None, Some(p)) => p.split(' ').map(String::from).collect(),
        (None, None) => return Vec::new(),
    };
    flags
}

fn how(proved: bool, blocked: Option<String>) -> String {
    match (proved, blocked) {
        (true, _) => "proved".to_string(),
        (false, Some(why)) => format!("checked: {why}"),
        (false, None) => "checked".to_string(),
    }
}

fn arity(owner: Option<i64>, seeded: bool) -> i64 {
    match (owner, seeded) {
        (Some(o), true) => o + 1,
        _ => 0,
    }
}

fn copied(built: Option<String>, dir: Option<String>) -> Option<String> {
    match (built, dir) {
        (Some(built), Some(_)) => Some(format!("{built}!")),
        (built, _) => built,
    }
}

fn kind(ext: Option<String>, stem: Option<String>) -> i64 {
    match (ext, stem) {
        (Some("out"), Some(stem)) => stem.len() as i64,
        _ => 0,
    }
}

fn named(item: &Item) -> bool {
    match item { Item::Fn { name: Some(name), .. } => name != "main", Item::Const { .. } => false, _ => true }
}

fn declared(shape: Shape, path: Option<String>) -> String {
    match (shape, path) {
        (Shape::Rust, _) => "rust".to_string(),
        (Shape::Path, Some(path)) => path,
        _ => "nikaia".to_string(),
    }
}
