//! The package's own Rust, which its Nikaia code calls (D24).

/// Twice a number.
pub(crate) fn twice(x: i64) -> i64 {
    x * 2
}

/// Three times a number.
pub(crate) fn thrice(x: i64) -> i64 {
    x * 3
}

/// The `.nika` file and line for a generated line.
pub(crate) fn lookup(table: &[(u32, &'static str, u32)], generated: u32) -> Option<(&'static str, u32)> {
    table
        .binary_search_by_key(&generated, |(line, _, _)| *line)
        .ok()
        .map(|at| (table[at].1, table[at].2))
}
