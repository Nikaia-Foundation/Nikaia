fn probe(xs: &[String]) -> i64 {
    let f = |a: &String, b| a.len() as i64 + b;
    let g = |a: &String| -> bool { a.is_empty() };
    let h = |a| a + 1;
    if g(&xs[0]) {
        return h(1);
    }
    f(&xs[0], 1)
}
