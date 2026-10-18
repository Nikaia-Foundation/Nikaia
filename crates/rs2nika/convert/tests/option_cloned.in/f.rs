fn first(xs: &[String], ns: &[u32]) -> Option<String> {
    let a = xs.first().cloned();
    let n = ns.last().copied();
    a
}
