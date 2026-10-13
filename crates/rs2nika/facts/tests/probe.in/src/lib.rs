use std::collections::HashMap;
pub fn f(v: Vec<u32>, m: HashMap<String,u16>, k: &str) -> usize {
    let a = v.get(0);
    let b = m.get(k);
    let n = v.len() + a.map(|x| *x as usize).unwrap_or(0);
    n + b.copied().unwrap_or(0) as usize
}
