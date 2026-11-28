//! The Rayon twin of `benches/par-map/allocates.nika` (ADR-async-v6 D28): the
//! same list, the same chain written as Rayon's `par_iter().map().collect()`,
//! the same sum printed. Kept in step with the `.nika` by hand.
use rayon::prelude::*;

fn main() {
    let n: i64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(1_000_000);
    let xs: Vec<i64> = (0..n).collect();
    let ys: Vec<String> = xs.par_iter().map(|x| format!("{}", x)).collect();
    let total: usize = ys.iter().map(|y| y.len()).sum();
    println!("{} {}", ys.len(), total);
}
