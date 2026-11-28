//! The Rayon twin of `benches/par-map/throws.nika` (ADR-async-v6 D28): the
//! same list, the same lambda that can fail, written as Rayon's
//! `par_iter().map().collect::<Result<Vec<_>, _>>()`, the same sum printed.
//! Kept in step with the `.nika` by hand. Rayon's collect into a `Result`
//! stops at a failure and keeps one; no element fails here, so both sides do
//! the same work.
use rayon::prelude::*;

#[derive(Debug)]
enum Bad {
    Negative(i64),
}

impl std::fmt::Display for Bad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bad::Negative(n) => write!(f, "element {} is negative", n),
        }
    }
}

impl std::error::Error for Bad {}

fn work(n: i64) -> Result<i64, Bad> {
    if n < 0 {
        return Err(Bad::Negative(n));
    }
    Ok(n * 3 + 1)
}

fn main() -> Result<(), Bad> {
    let n: i64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(1_000_000);
    let xs: Vec<i64> = (0..n).collect();
    let ys: Vec<i64> = xs.par_iter().map(|x| work(*x)).collect::<Result<_, _>>()?;
    let total: i64 = ys.iter().sum();
    println!("{} {}", ys.len(), total);
    Ok(())
}
