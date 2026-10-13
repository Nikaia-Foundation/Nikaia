fn probe(xs: &[String], m: &HashMap<String, i64>) -> Option<i64> {
    let mut n = 0;
    for x in xs {
        let w = match m.get(x) {
            Some(w) => w,
            None => continue,
        };
        let v = match m.get(&w.to_string()) {
            Some(v) => v,
            None => return None,
        };
        n += v + w;
    }
    Some(n)
}
