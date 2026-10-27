//! R103-R105: `const` -> `comptime`, `const fn`, a `const` of an `impl`.
const LIMIT: usize = 4096;
pub const NAME: &str = "x";
pub(crate) const NAMES: &[&str] = &["a", "b"];
const SIZES: [u64; 3] = [1, 2, 3];
const MASK: u32 = 0xff;
const NEG: i64 = -1;
const FLAG: bool = true;
const C: char = 'a';
const KEYWORDS: [&str; 3] = [
    "fn",
    "let",
    "if",
];
const STD: &str = include_str!("../std.contracts");
const HAND: &[(&str, &[&str])] = &[
    ("a", &["b"]),
];
pub(crate) const RT: &[&str] = &[
    "x86",
    "arm",
];
const LIMIT_B: u128 = 1 << 60;
const PAIRS: &[(&str, usize)] = &[("a", 1), ("b", 2)];
const SEP: char = ',';
const HULLS: [&str; 2] = [NAME, NAME];

pub struct Foo {
    pub a: usize,
}

impl Foo {
    pub const MAX: usize = 10;
    const PREFIX: &'static str = "nk";
    const LIMIT: usize = 2;
    pub fn get(&self) -> usize {
        Self::MAX + self.a + Self::PREFIX.len()
    }
}

const fn twice(x: u64) -> u64 {
    x * 2
}

pub fn run() -> usize {
    const INNER: usize = 3;
    LIMIT + INNER + NAMES.len() + Foo::MAX
}
