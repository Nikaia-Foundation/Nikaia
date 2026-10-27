fn take(at: &mut BTreeMap<i64, i64>, k: i64) -> i64 {
    if let Some(v) = at.remove(&k) {
        return v + 1;
    }
    0
}

fn take_or(at: &mut BTreeMap<i64, i64>, k: i64) -> i64 {
    let mut n = 0;
    if let Some(v) = at.remove(&k) {
        n = v;
    } else {
        n = 7;
    }
    n
}

fn first(xs: &mut Vec<i64>) -> i64 {
    if let Some(top) = xs.pop() {
        if let Some(next) = xs.pop() {
            return top + next;
        }
        return top;
    }
    -1
}

fn long(o: Option<String>) -> bool {
    o.is_some_and(|s| s.len() > 2)
}

fn big(o: Option<i64>) -> bool {
    o.is_some_and(positive)
}

fn positive(n: i64) -> bool {
    n > 0
}
