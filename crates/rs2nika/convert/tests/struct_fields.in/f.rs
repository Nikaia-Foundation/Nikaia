pub struct Seen<'a> {
    pub names: Vec<String>,
    pub(crate) count: usize,
    // a comment stays
    map: HashMap<String, Vec<u32>>,
    best: Option<Box<Ty>>,
    lit: &'a str,
}
pub(crate) struct Pair<T> {
    a: T,
}
