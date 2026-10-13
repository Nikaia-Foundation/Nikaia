//! `std::cli` - the command line.

/// The arguments the program was started with, the program's own name first,
/// exactly as Part III describes it.
pub fn args() -> Args {
    Args(std::env::args())
}

/// The arguments, read in order. **A position is an `i64`**, as every count
/// in the language is: `cli::args().nth(at)` with `at: i64` is what the ledger
/// describes (`Args::nth`), and Rust's own `nth` takes a `usize`.
pub struct Args(std::env::Args);

impl Args {
    /// The argument at a position, or nothing where there is none (or the
    /// position is negative).
    // Not `Iterator::nth`: that one counts in `usize`.
    #[allow(clippy::should_implement_trait)]
    pub fn nth(&mut self, at: i64) -> Option<String> {
        let at = usize::try_from(at).ok()?;
        self.0.nth(at)
    }
}

impl Iterator for Args {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

#[cfg(test)]
mod tests {
    /// `cli::args().nth(at)` takes the `i64` the language counts in.
    #[test]
    fn a_position_is_an_i64() {
        let at: i64 = 0;
        assert!(super::args().nth(at).is_some());
        assert_eq!(super::args().nth(-1), None);
        assert_eq!(super::args().nth(i64::MAX), None);
    }
}
