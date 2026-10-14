fn give() -> i64 {
    1
}

fn other() -> String {
    let n = give();
    format!("give() said {n}")
}
