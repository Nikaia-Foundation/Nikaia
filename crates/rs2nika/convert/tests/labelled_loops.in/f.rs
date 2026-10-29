fn first(m: &BTreeMap<&str, Vec<u32>>) -> Option<u32> {
    let mut found = None;
    'packages: for (name, units) in m {
        for u in units {
            if *u > 3 {
                found = Some(*u);
                break 'packages;
            }
            if *u == 0 {
                continue 'packages;
            }
        }
    }
    found
}
fn deep(xs: &Vec<Vec<Vec<i64>>>) -> i64 {
    let mut n = 0;
    'outer: for a in xs {
        for b in a {
            'inner: loop {
                for c in b {
                    if *c < 0 { continue 'outer; }
                    if *c == 0 { break 'inner; }
                    if *c > 9 { break 'outer; }
                    n += c;
                }
                break;
            }
            n += 1;
        }
    }
    n
}
