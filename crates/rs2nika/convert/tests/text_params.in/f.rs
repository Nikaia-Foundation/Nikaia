pub struct Word<'a> {
    text: &'a str,
}
fn width(text: &str, owned: &String, n: usize) -> usize {
    text.len() + owned.len() + n
}
fn head<'a>(line: &'a str) -> &'a str {
    line.trim()
}
fn fallback(name: Option<&str>) -> usize {
    name.unwrap_or("").len()
}
fn any_of(xs: &[String], f: impl Fn(&str) -> bool) -> bool {
    xs.iter().any(|x| f(x))
}
