//! **`u64`, `u32` and the bit operators**
//! ([ADR-285](../../../docs/specification/adr/adr-285.md)): the types a program
//! asked for, `&`, `|`, `^`, `<<`, `>>` and `!` on integers, a literal as wide
//! as a `u64`, and a text's bytes. The program that asked is a hash; it and what
//! each operator computes are `tests/language/src/bits.nika`, and here is what
//! is refused.

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check(&parsed, &own, &library)
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect()
}

/// **The refusals, each in this language's words** (D1, D3, D4): a bit
/// operator beside a comparison without parentheses, with the parenthesised
/// line handed over; `&` on two `bool`s, pointed at `&&`; and a `u64` added to
/// an `i64`.
#[test]
fn what_is_refused_is_refused_with_the_line_to_write() {
    let beside = findings(
        "fn main() {\n\
         \x20   let a: u64 = 5\n\
         \x20   let zero = a & 1 == 0\n\
         \x20   let fine = (a & 1) == 0\n\
         }\n",
    );
    assert!(
        beside
            .iter()
            .any(|f| f.code == "NK1197" && f.help.as_deref() == Some("Write `(a & 1) == 0`.")),
        "{beside:#?}"
    );
    assert_eq!(
        beside.iter().filter(|f| f.code == "NK1197").count(),
        1,
        "the parenthesised line is not refused: {beside:#?}"
    );

    let booleans = findings("fn main() {\n    let both = true & false\n}\n");
    assert!(
        booleans
            .iter()
            .any(|f| f.code == "NK1198" && f.help.as_deref() == Some("`&&` joins two `bool`s")),
        "{booleans:#?}"
    );

    let mixed = findings(
        "fn main() {\n\
         \x20   let a: u64 = 5\n\
         \x20   let b: i64 = 6\n\
         \x20   let c = a + b\n\
         }\n",
    );
    assert!(
        mixed.iter().any(|f| f.code == "NK1199"
            && f.message == "You're mixing a `u64` and an `i64` in one operation."),
        "{mixed:#?}"
    );
}

/// **A literal as wide as a `u64` holds** (D2, run in `bits.nika`), and no
/// wider: above an `i64` it is a `u64`'s, and refused against `i64` where no
/// type stands beside it.
#[test]
fn a_literal_above_an_i64_is_a_u64s() {
    let bare = findings("fn main() {\n    let big = 18446744073709551615\n}\n");
    assert!(
        bare.iter().any(|f| f.code == "NK1116"
            && f.help
                .as_deref()
                .is_some_and(|h| h.contains("`let x: u64 = …`"))),
        "{bare:#?}"
    );
    assert!(parse_to_ast("fn main() {\n    let x: u64 = 18446744073709551616\n}\n").is_err());
}

/// **An unannotated constant takes its type from its uses**
/// ([ADR-285](../../../docs/specification/adr/adr-285.md)): with nothing
/// pinning `a`, the three names are one number, no use asks, and what they are
/// given does not fit an `i32` - so the sum is an `i64` (run in `bits.nika`).
/// Beside an `i32` `a`, `b` is asked to be an `i32` by `a + b`, and `3000000000` does not fit one:
/// `NK1116` at `b`, naming the use, where it was `rustc`'s *cannot add `i64` to
/// `i32`*.
#[test]
fn a_large_constant_is_an_i64_and_mixes_with_nothing_else() {
    let pinned = findings(
        "fn main() {\n\
         \x20   let a: i32 = 1\n\
         \x20   let b = 3000000000\n\
         \x20   let c = a + b\n\
         }\n",
    );
    assert!(
        pinned.iter().any(|f| f.code == "NK1116"
            && f.message == "`3000000000` doesn't fit in an `i32`."
            && f.notes
                .iter()
                .any(|n| n.contains("`b` is an `i32` because of how it is used"))),
        "{pinned:#?}"
    );
    assert_eq!(pinned.len(), 1, "one cause, one refusal: {pinned:#?}");
}

/// **A shift by a constant count outside `0..<width` is `NK1116`** (#653,
/// Part I 2.2): it is certain to stop the program, so it is refused before
/// the language below sees it, for `i64`, `u8` and `i32`, and for a negative
/// count. A count inside the width passes.
#[test]
fn a_shift_past_the_width_is_refused() {
    for (source, ty, count, width) in [
        (
            "fn main() {\n    let n: i64 = 1\n    let m = n << 64\n    println(f\"{m}\")\n}\n",
            "an `i64`",
            "64",
            "64",
        ),
        (
            "fn main() {\n    let n = 1\n    let m = n >> 70\n    println(f\"{m}\")\n}\n",
            "an `i32`",
            "70",
            "32",
        ),
        (
            "fn main() {\n    let b: u8 = 1\n    let m = b << 8\n    println(f\"{m}\")\n}\n",
            "a `u8`",
            "8",
            "8",
        ),
        (
            "fn main() {\n    let c: i32 = 1\n    let m = c >> 32\n    println(f\"{m}\")\n}\n",
            "an `i32`",
            "32",
            "32",
        ),
        (
            "fn main() {\n    let c: i32 = 1\n    let m = c << -1\n    println(f\"{m}\")\n}\n",
            "an `i32`",
            "-1",
            "32",
        ),
    ] {
        let found = findings(source);
        let message = format!("This shifts {ty} by {count} places, and it has {width} bits.");
        assert!(
            found
                .iter()
                .any(|f| f.code == "NK1116" && f.message == message),
            "{source}: {found:#?}"
        );
    }
    for source in [
        "fn main() {\n    let n: i64 = 1\n    let m = n << 63\n    println(f\"{m}\")\n}\n",
        "fn main() {\n    let b: u8 = 1\n    let m = b << 7\n    println(f\"{m}\")\n}\n",
        "fn main() {\n    let c: i32 = 1\n    let s = 3\n    let m = c >> s\n    println(f\"{m}\")\n}\n",
    ] {
        let found = findings(source);
        assert!(found.is_empty(), "{source}: {found:#?}");
    }
}

/// **An open number is shifted as the type its uses decide** (#656, Part I
/// 2.4): `let n = 1` is an `i32` where nothing asks, so `n << 40` is
/// `NK1116`, and so is `1 << 40` given to a name an `i32` use asks for; where
/// a use asks for an `i64`, both shift by 40 places.
#[test]
fn an_open_number_is_shifted_as_the_type_it_gets() {
    for source in [
        "fn main() {\n    let n = 1\n    let m = n << 40\n    println(f\"{m}\")\n}\n",
        "fn main() {\n    let m = 1 << 40\n    let w: i32 = m\n    println(f\"{w}\")\n}\n",
        "fn main() {\n    let m: i32 = 1 << 40\n    println(f\"{m}\")\n}\n",
    ] {
        let found = findings(source);
        let message = "This shifts an `i32` by 40 places, and it has 32 bits.";
        assert!(
            found
                .iter()
                .any(|f| f.code == "NK1116" && f.message == message),
            "{source}: {found:#?}"
        );
        assert_eq!(found.len(), 1, "{source}: {found:#?}");
    }
    for source in [
        "fn main() {\n    let n = 1\n    let m: i64 = n << 40\n    println(f\"{m}\")\n}\n",
        "fn main() {\n    let m = 1 << 40\n    let w: i64 = m\n    println(f\"{w}\")\n}\n",
        "fn main() {\n    let n = 3000000000\n    let m = n << 4\n    println(f\"{m}\")\n}\n",
        "fn main() {\n    let n = 1\n    let m = n << 31\n    println(f\"{m}\")\n}\n",
    ] {
        let found = findings(source);
        assert!(found.is_empty(), "{source}: {found:#?}");
    }
}

/// **A shift written only in literals is folded exactly** (#661, ADR-285
/// D40): it takes the first type that holds its value, so `1 << 40` and
/// `1 << 31` given to nothing, or to a name nothing asks of, are `i64`s and
/// pass; `1 << 63` no type holds, and it is `NK1116` in this language's words
/// rather than the language below's. Where a use asks for an `i32`, the shift
/// is that type's: `1 << 31` passes, and its bits are lost there.
#[test]
fn a_shift_of_literals_is_folded_exactly() {
    for source in [
        "fn main() {\n    println(1 << 40)\n}\n",
        "fn main() {\n    if (1 << 40) > 0 {\n        println(\"big\")\n    }\n}\n",
        "fn main() {\n    println(1 << 31)\n}\n",
        "fn main() {\n    println(-(1 << 31))\n}\n",
        "fn main() {\n    let m = 1 << 40\n    println(m)\n}\n",
        "fn main() {\n    let m = (1 << 40) + 2\n    println(f\"{m}\")\n}\n",
        "fn main() {\n    let m: i32 = 1 << 31\n    println(m)\n}\n",
        "fn main() {\n    let m = 1 << 31\n    let w: i32 = m\n    println(w)\n}\n",
        "fn main() {\n    let m: i64 = 1 << 63\n    println(m)\n}\n",
    ] {
        let found = findings(source);
        assert!(found.is_empty(), "{source}: {found:#?}");
    }
    for (source, message) in [
        (
            "fn main() {\n    println(1 << 63)\n}\n",
            "This comes to 9223372036854775808, which doesn't fit in an `i64`.",
        ),
        (
            "fn main() {\n    let m = 1 << 63\n    println(m)\n}\n",
            "This comes to 9223372036854775808, which doesn't fit in an `i64`.",
        ),
        (
            "fn main() {\n    println(1 << 70)\n}\n",
            "This shifts an `i64` by 70 places, and it has 64 bits.",
        ),
        (
            "fn main() {\n    let m: u8 = 1 << 8\n    println(m)\n}\n",
            "This shifts a `u8` by 8 places, and it has 8 bits.",
        ),
    ] {
        let found = findings(source);
        assert!(
            found
                .iter()
                .any(|f| f.code == "NK1116" && f.message == message),
            "{source}: {found:#?}"
        );
        assert_eq!(found.len(), 1, "{source}: {found:#?}");
    }
}
