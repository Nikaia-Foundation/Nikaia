fn named(item: &Item) -> String {
    let Item::Fn { name, .. } = item else {
        return String::new();
    };
    name.map(|n| resolve(n)).unwrap_or_default()
}

fn part(c: &Crossing) -> Option<(String, String)> {
    c.refused_part().map(|part| (part, c.refused_at()))
}

fn other(r: Result<u8, E>) -> Result<u8, E> {
    r.map(|v| v + 1)
}
