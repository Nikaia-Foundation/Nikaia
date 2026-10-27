fn port(n: i64) -> String {
    let p = half(n).ok();
    show(p)
}

fn report(n: i64) {
    if let Ok(v) = half(n) {
        println!("got {v}");
    }
}

fn quiet(n: i64) {
    match half(n) {
        Ok(v) => println!("half {v}"),
        Err(_) => {}
    }
}

fn touched(n: i64) {
    if let Ok(_) = half(n) {
        println!("even");
    }
}

fn even(n: i64) -> bool {
    half(n).is_ok()
}

fn odd(n: i64) -> bool {
    half(n).is_err()
}

fn not_odd(n: i64) -> bool {
    !half(n).is_err()
}

fn four(n: i64) -> bool {
    half(n).is_ok_and(|v| v == 4)
}

fn kept(n: i64) -> Option<i64> {
    match half(n) {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

fn quartered(n: i64) -> String {
    let q = match half(n) {
        Ok(v) => v / 2,
        Err(_) => 0,
    };
    format!("{q}")
}
