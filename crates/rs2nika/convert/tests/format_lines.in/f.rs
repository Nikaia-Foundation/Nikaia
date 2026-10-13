fn report(path: &str, rows: &[Held]) -> String {
    format!(
        "asserts in {path}: {} - proved {}, \
         refused {}, {}\n",
        rows.len(),
        count(|h| matches!(h, Held::Proved)),
        count(|h| matches!(h, Held::Refused)),
        if rows.len() > 2 { "many" } else { "few, some" },
    )
}

fn short(a: i64) -> String {
    format!("{} and {}", a)
}
