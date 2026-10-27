fn probe(xs: &[String], key: &str) -> usize {
    let mut n = 0;
    for x in xs {
        match x.find(key) {
            Some(at) => {
                n += at;
                n += 1;
            }
            None => continue,
        }
    }
    n
}

fn second(key: &str) -> usize {
    match lookup(key) {
        None => return 0,
        Some(v) => {
            let k = v.len();
            k + 1
        }
    }
}

fn third(xs: &[String]) -> Option<usize> {
    let mut n = 0;
    for x in xs {
        match lookup(x) {
            Some(v) => n += v.len(),
            _ => break,
        }
        match lookup(x) {
            None => continue,
            Some(w) => {
                if w.is_empty() {
                    n += 1;
                }
            }
        }
    }
    match lookup("a") {
        Some(v) => Some(v.len() + n),
        None => return None,
    }
}
