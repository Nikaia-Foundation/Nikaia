fn pick(o: Option<i64>, k: i64) -> i64 {
    match o {
        Some(0) => 1,
        Some(n) => n + k,
        None => panic!("no value for {}", k),
    }
}

fn guard_n(n: i64) -> i64 {
    if n < 0 {
        panic!("negative: {n}");
    }
    if n > 100 {
        panic!("too big");
    }
    n
}

fn half(n: i64) -> i64 {
    match n % 2 {
        0 => n / 2,
        1 => n,
        _ => unreachable!(),
    }
}

fn positive(n: i64) -> i64 {
    if n > 0 {
        return n;
    }
    panic!("not positive: {}", n)
}
