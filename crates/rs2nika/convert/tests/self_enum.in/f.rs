enum Token {
    Word(String),
    Count(usize),
    List(Vec<usize>),
    End,
}

struct Pt {
    x: i64,
}

impl Pt {
    fn make() -> Self {
        Self { x: 0 }
    }

    fn twice() -> Self {
        let p = Self::make();
        p
    }
}

fn tail(s: &str) -> String {
    let rest = &s[2..];
    let all = &s[..];
    format!("{rest}{all}")
}
