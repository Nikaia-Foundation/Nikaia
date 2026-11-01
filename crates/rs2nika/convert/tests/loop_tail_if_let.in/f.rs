fn a(xs: &[Option<i64>]) -> i64 {
    let mut n = 0;
    for x in xs {
        n += 1;
        if let Some(v) = x {
            n += *v;
        }
    }
    n
}
fn b(ls: &[String]) {
    let mut i = 0;
    while i < ls.len() {
        i += 1;
        if let Some((k, v)) = ls[i].split_once("=") {
            println!("{} {}", k, v);
        }
    }
}
fn c(xs: &[Option<i64>]) {
    for x in xs {
        if let Some(v) = x {
            println!("{}", v);
        }
        println!("end");
    }
}
fn d(xs: &[Option<i64>]) {
    for x in xs {
        if let Some(v) = x {
            println!("{}", v);
        } else {
            println!("none");
        }
    }
}
