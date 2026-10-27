fn chain_probe(items: &[Item], key: &str, bad: &[String]) -> i64 {
    let mut n = 0;
    for item in items {
        if let Shape::Circle { r, .. } = item.shape && let Some(v) = item.extra {
            n += r + v;
        }
        if item.ok && let Shape::Square(side) = item.shape && side > 2 {
            n += side;
        }
        if item.ok && let Some((target, _)) = key.split_once("::") && !bad.iter().any(|b| b == target) {
            n += target.len() as i64;
        }
        if let Shape::Square(s) = item.shape && s > 0 && let Some(e) = item.extra {
            n += s * e;
        }
        if let Shape::Square(s) = item.shape && s > 1 {
            n += 1;
        } else {
            n -= 1;
        }
    }
    n
}
