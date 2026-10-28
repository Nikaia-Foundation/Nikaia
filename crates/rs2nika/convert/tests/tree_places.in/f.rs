use std::collections::HashMap;

#[derive(Default)]
struct Tally {
    seen: HashMap<String, i64>,
    names: Vec<String>,
    total: i64,
}

struct Walk<'a> {
    names: &'a [String],
    tally: Tally,
}

fn count(names: &[String]) -> Tally {
    let walk = Walk { names, tally: Tally::default() };
    walk.tally
}

fn grow(views: &mut Vec<Tally>, from: usize) {
    for view in views.iter_mut().skip(from) {
        view.total += 1;
    }
}

fn says(count: i64) -> String {
    let msg = format!(
        "a message long enough that the syntax tree's dump shortens it: {count}"
    );
    msg
}
