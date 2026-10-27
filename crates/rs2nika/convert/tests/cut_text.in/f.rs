fn parse(name: &str) -> Result<i64> {
    match name {
        "one" => Ok(1),
        _ => Err(refused!(
            "There's no target called `{name}`. Use one, \
    }
}

fn kept(x: i64) -> i64 {
    x + 1
}

fn whole(name: &str) -> String {
    format!(
        "no `{name}` here, \
         try another"
    )
}
