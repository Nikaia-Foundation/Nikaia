fn first_word(text: &str) -> Option<&str> {
    text.split(' ').next()
}

fn arms(binary: String, at: Option<usize>, shape: Option<Shape>) -> usize {
    let binary = match binary.rsplit_once('-') {
        Some((name, hash)) if hash.len() > 3 => name.to_string(),
        _ => binary,
    };
    let width = match at {
        Some(n) => n + 1,
        None => 0,
    };
    let dot = match shape {
        Some(Shape::Dot(x)) => x,
        _ => 0,
    };
    width + dot + binary.len()
}

fn paths(table: &Table, n: u32, name: Option<&str>, kept: &Option<String>) -> usize {
    let path = table.get("path").and_then(Value::as_str);
    let c = u32::try_from(n).ok().and_then(char::from_u32);
    let size = name.map(|n| n.len() + 1).unwrap_or(0);
    let kept_len = kept.as_ref().map(|k| k.len() * 2).unwrap_or(0);
    let first = first_word("a b").map(|w| w.len() + 2);
    let found = table.get("x");
    let x = found.map_or(0, |f| f.len() + 3);
    let y = found.map_or(0, str::len);
    size + kept_len + x
}

fn chains(dependency: Option<Dep>, recorded: Option<String>, ready: bool) -> usize {
    if let Some(dependency) = dependency && let Some(shipped) = shipped_ledger(dependency.root) && shipped.fresh() {
        return 1;
    }
    if let Some(entry) = recorded && let Some(asked) = read(entry) {
        return asked;
    }
    if ready && let Some(a) = lookup(1) && let Some(b) = lookup(a) {
        return a + b;
    }
    0
}
