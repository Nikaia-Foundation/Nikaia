pub fn parse(name: &str) -> Result<Target> {
    match known(name) {
        Some(k) => Ok(k),
        None => Err(refused!(
            "There's no target called `{name}`. Use x86_64-linux, \
             or a triple from `rustc --print target-list`."
        )),
    }
}

pub fn check(n: i64) -> Result<()> {
    let e = refused!("too many: {n}");
    Err(e)
}
