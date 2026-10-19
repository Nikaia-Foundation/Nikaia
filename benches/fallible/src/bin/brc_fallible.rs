//! 1BRC's aggregation on fallible collections: the same steps as
//! `brc_std.rs`, with every table, list and text taken from a budget that may
//! refuse (ADR-327 D5).

#[path = "../budget.rs"]
mod budget;

use budget::{Budget, List, Map};
use std::fmt::Write;

struct Stats {
    min: i64,
    max: i64,
    sum: i64,
    count: i64,
}

fn tenths(field: &[u8]) -> i64 {
    let (negative, digits) = match field.first() {
        Some(b'-') => (true, &field[1..]),
        _ => (false, field),
    };
    let mut n = 0i64;
    for &b in digits {
        if b != b'.' {
            n = n * 10 + (b - b'0') as i64;
        }
    }
    if negative {
        -n
    } else {
        n
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("the measurements file");
    let data = std::fs::read(path).expect("read the measurements");
    let mut stations: Map<&[u8], Stats> = Map::with_hasher_in(Default::default(), Budget);
    for line in data.split(|&b| b == b'\n') {
        let Some(at) = line.iter().position(|&b| b == b';') else {
            continue;
        };
        let t = tenths(&line[at + 1..]);
        if stations.len() == stations.capacity() && stations.try_reserve(1).is_err() {
            budget::spent()
        }
        let stats = stations.entry(&line[..at]).or_insert(Stats {
            min: t,
            max: t,
            sum: 0,
            count: 0,
        });
        stats.min = stats.min.min(t);
        stats.max = stats.max.max(t);
        stats.sum += t;
        stats.count += 1;
    }
    let mut names: List<&[u8]> = List::new_in(Budget);
    for name in stations.keys() {
        budget::push(&mut names, *name);
    }
    names.sort_unstable();
    let mut out = budget::text();
    for name in names {
        let s = &stations[name];
        let mean = mean_tenths(s.sum, s.count);
        budget::extend(
            &mut out,
            std::str::from_utf8(name).expect("UTF-8").as_bytes(),
        );
        let _ = writeln!(
            budget::Into(&mut out),
            "={}{}.{}/{}{}.{}/{}{}.{}",
            sign(s.min),
            s.min.unsigned_abs() / 10,
            s.min.unsigned_abs() % 10,
            sign(mean),
            mean.unsigned_abs() / 10,
            mean.unsigned_abs() % 10,
            sign(s.max),
            s.max.unsigned_abs() / 10,
            s.max.unsigned_abs() % 10
        );
    }
    print!("{}", std::str::from_utf8(&out).expect("UTF-8"));
}

/// The official rounding (the Java baseline's `Math.round(x * 10.0) / 10.0`):
/// a tie goes toward +infinity. `sum` is in tenths, so the mean in tenths is
/// `floor((2 * sum + count) / (2 * count))`, exact and with no float.
fn mean_tenths(sum: i64, count: i64) -> i64 {
    (2 * sum + count).div_euclid(2 * count)
}

/// The sign of a value in tenths as printed: none for zero, so never `-0.0`.
fn sign(t: i64) -> &'static str {
    if t < 0 {
        "-"
    } else {
        ""
    }
}
