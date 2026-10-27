fn shown(xs: &[Unit], written: Option<String>) -> Vec<String> {
    xs.iter().map(|u| match written { Some(w) if u.first => w.clone(), _ => u.path.clone() }).collect()
}
fn dir(v: Option<String>) -> Option<String> {
    let t = match v { Some(_) => None, None => { Some(default_dir()) } };
    t
}
fn pair(a: Option<i64>, b: Option<i64>) -> i64 {
    let n = match (a, b) { (Some(x), _) => x, (None, _) => 0 };
    n
}
