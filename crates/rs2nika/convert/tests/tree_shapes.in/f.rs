struct Around { foreign: Vec<String>, newly: bool }
struct P { site: Option<(u32, String)> }

fn split(around: Around) -> usize {
    let Around { foreign, newly: mut fresh } = around;
    fresh = !fresh;
    if fresh { foreign.len() } else { 0 }
}

fn split_made() -> bool {
    let Around { newly, .. } = make();
    newly
}

fn what(pause: Option<&P>) -> Option<String> {
    let what = pause
        .and_then(|pause| pause.site.as_ref())
        .map(|(_, what)| what.clone());
    what
}

fn kept(xs: &[i64]) -> usize {
    let found: Vec<_> = xs.iter().filter(|x| **x > 1).collect();
    sync::count(found)
}

fn panics(t: Target) -> String {
    crate::project_settings::panic_strategy(t)
}

fn key(t: &str) -> Result<String> {
    Ok(t.to_string())
}

fn keyed(t: &str) -> (String, OsString) {
    (key(t).unwrap_or_default(), OsString::from(t))
}

fn long_list(a: &str) -> Vec<(String, String)> {
    vec![
        (TARGET_VARIABLE_NAME.to_string(), a.to_string()),
        (TARGET_VARIABLE_NAME.to_string(), a.to_string()),
        (TARGET_VARIABLE_NAME.to_string(), a.to_string()),
        (TARGET_VARIABLE_NAME.to_string(), a.to_string()),
    ]
    .into_iter()
    .collect()
}

fn both(a: &str) -> (String, String) {
    (a.to_string(), a.to_string())
}

fn level(word: &str) -> String {
    match word {
        "error" | "warn" => word.to_string(),
        _ => String::new(),
    }
}
