enum Reached {
    Opaque(Option<String>),
    Pair(Option<String>, i64),
    Method,
}

fn shown(r: Reached) -> String {
    match r {
        Reached::Opaque(None) => "none".to_string(),
        Reached::Opaque(Some(name)) => format!("@{name}"),
        Reached::Pair(None, _) => "no pair".to_string(),
        Reached::Pair(Some(a), n) => format!("{a} {n}"),
        Reached::Method => "m".to_string(),
    }
}

fn late(r: Reached) -> String {
    match r {
        Reached::Opaque(Some(name)) => name,
        Reached::Method => "m".to_string(),
        Reached::Opaque(None) => "none".to_string(),
        _ => "other".to_string(),
    }
}

fn kept(r: Reached) -> String {
    match r {
        Reached::Pair(None, 1) => "one".to_string(),
        Reached::Pair(Some(a), n) => format!("{a} {n}"),
        _ => "other".to_string(),
    }
}
