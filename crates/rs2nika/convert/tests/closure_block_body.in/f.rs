fn f(params: &Vec<(String, i64)>, index: usize) -> bool {
    params.get(index).is_some_and(|(name, n)| { *n > 0 && name.len() > 1 })
}
fn g(x: Option<i64>) -> i64 {
    x.unwrap_or_else(|| { 7 })
}
