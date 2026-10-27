//! R100-R102: a `use` inside a block, and `static`.
use std::collections::BTreeMap;
use crate::emit::{Lowered, Needs};

static NAMES: &[&str] = &["a", "b", "c"];
pub static LIMIT: u64 = 4096;
pub(crate) static TITLE: &'static str = "t";
static SIZES: [usize; 2] = [1, 2];
static COUNT: AtomicU64 = AtomicU64::new(0);
static ASKED: std::sync::Mutex<BTreeMap<String, String>> =
    std::sync::Mutex::new(BTreeMap::new());
static PARSED: std::sync::OnceLock<Ledger> = std::sync::OnceLock::new();
thread_local! {
    static SHOWN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn run(x: u64) -> u64 {
    use std::io::Read;
    use crate::prove::ClaimRun;
    use std::collections::BTreeMap;
    use crate::contracts::sync::{
        visit_expr, visit_stmt,
    };
    let a = x + LIMIT;
    a
}

fn again(x: u64) -> u64 {
    use crate::prove::ClaimRun;
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::run;
    use super::again as twice;
    use crate::emit::Lowered;

    #[test]
    fn t() {
        assert_eq!(run(1), 4097);
    }
}
