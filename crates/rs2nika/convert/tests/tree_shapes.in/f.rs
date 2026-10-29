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
