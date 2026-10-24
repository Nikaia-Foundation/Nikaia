//! **A bound on a list's values, and `--optimization=remove-overflow-checks`**
//! ([ADR-306](../../../docs/specification/adr/adr-306.md)).
//!
//! Every program here is lowered with both options off and both at
//! `aggressive`, compiled with overflow checks on and without `-O`, and run:
//! it prints the same, or stops at the same place with the language's message,
//! at both (ADR-306 D2). The cases that stop are the ones a bound that says
//! more than the program does would get wrong.

use crate::common;
use std::process::Command;

use nikaia::bounds::{BoundsChecks, OverflowChecks};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

#[derive(Debug, Clone, Copy)]
enum Level {
    Off,
    Aggressive,
}

const LEVELS: [Level; 2] = [Level::Off, Level::Aggressive];

fn lowered(source: &str, level: Level) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    let how = match level {
        Level::Off => Build::default(),
        Level::Aggressive => Build {
            bounds: BoundsChecks::Removed,
            overflow: OverflowChecks::Removed,
            ..Build::default()
        },
    };
    emit_program(&parsed, how).expect("it lowers").rust
}

/// Indexes written without their check.
fn unchecked_indexes(rust: &str) -> usize {
    rust.matches("nikaia_std::proven::read(").count()
        + rust.matches("nikaia_std::proven::write(").count()
}

/// Operations written without their overflow check.
fn unchecked_arithmetic(rust: &str) -> usize {
    rust.matches("unsafe { nikaia_std::proven::add::<").count()
        + rust.matches("unsafe { nikaia_std::proven::sub::<").count()
        + rust.matches("unsafe { nikaia_std::proven::mul::<").count()
        + rust.matches("unsafe { nikaia_std::proven::shl::<").count()
        + rust.matches("unsafe { nikaia_std::proven::shr::<").count()
}

fn run(purpose: &str, source: &str, level: Level) -> (bool, String, String) {
    let rust = lowered(source, level);
    let dir = common::scratch_dir(&format!("values-{purpose}-{level:?}"));
    let path = dir.join("program.rs");
    std::fs::write(&path, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &path,
        &[
            "--crate-type",
            "bin",
            "-C",
            "overflow-checks=on",
            "-o",
            &binary.to_string_lossy(),
        ],
    );
    assert!(
        compiled.status.success(),
        "{purpose} did not compile at {level:?}:\n{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary).output().expect("run it");
    std::fs::remove_dir_all(&dir).ok();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn prints(purpose: &str, source: &str, expected: &str) {
    for level in LEVELS {
        let (ok, out, err) = run(purpose, source, level);
        assert!(ok, "{purpose} failed at {level:?}:\n{err}");
        assert_eq!(out, expected, "{purpose} at {level:?}");
    }
}

/// Stops at both levels, with a message that contains `said` - never at a
/// debug assertion of a wrong proof.
fn stops(purpose: &str, source: &str, said: &str) {
    for level in LEVELS {
        let (ok, _, err) = run(purpose, source, level);
        assert!(!ok, "{purpose} ran to the end at {level:?}");
        assert!(
            !err.contains("a proved index is outside"),
            "{purpose}: a check was dropped that did not hold at {level:?}:\n{err}"
        );
        assert!(err.contains(said), "{purpose} at {level:?}:\n{err}");
    }
}

/// D1, D5: an operation the walk proves inside its type is written
/// `nikaia_std::proven::*::<T>`, naming the type (ADR-306 D19); nothing is
/// unchecked at `off`.
#[test]
fn an_operation_proved_inside_its_type_is_written_without_its_check() {
    let source = "\
fn main() {
    let n: u32 = 2000
    let mut last: u32 = 0
    for v in 0..<n {
        last = 2 * v + 1
    }
    println(f\"{last}\")
}
";
    assert_eq!(unchecked_arithmetic(&lowered(source, Level::Off)), 0);
    let rust = lowered(source, Level::Aggressive);
    assert_eq!(unchecked_arithmetic(&rust), 2, "{rust}");
    assert!(rust.contains("nikaia_std::proven::add::<u32>("), "{rust}");
    prints("proved-arithmetic", source, "3999\n");
}

/// An operation that does overflow keeps its check at every level.
#[test]
fn an_operation_that_overflows_stops_the_program_at_every_level() {
    let source = "\
fn double(x: u32) -> u32 {
    return x * 2
}

fn main() {
    println(f\"{double(3000000000)}\")
}
";
    assert_eq!(unchecked_arithmetic(&lowered(source, Level::Aggressive)), 0);
    stops("overflows", source, "overflow");
}

/// A guard is a fact: below it the operation fits.
#[test]
fn a_guard_proves_the_operation_under_it() {
    let source = "\
fn double(x: u32) -> u32 {
    if x < 1000 {
        return x * 2
    }
    return 0
}

fn main() {
    println(f\"{double(21)}\")
}
";
    assert_eq!(unchecked_arithmetic(&lowered(source, Level::Aggressive)), 1);
    prints("guarded", source, "42\n");
}

/// ADR-306 D18: a shift's count the walk proves inside `0..<64` - here from
/// `trailing_zeros`'s `0..=64` and the guard - is written as Rust's release
/// without its check as the shift that may assume it, `proven::shl`, and
/// so is the `28 - k` it is made of; at `off` both keep their checks.
#[test]
fn a_shift_proved_inside_its_width_is_written_without_its_check() {
    let source = "\
fn scaled(x: u64, y: u64) -> u64 {
    let k = x.trailing_zeros()
    if k > 28 {
        return 0
    }
    return y << (28 - k)
}

fn main() {
    println(f\"{scaled(16, 1)} {scaled(0, 1)} {scaled(1 << 20, 3)}\")
}
";
    assert_eq!(unchecked_arithmetic(&lowered(source, Level::Off)), 0);
    let rust = lowered(source, Level::Aggressive);
    assert!(rust.contains("nikaia_std::proven::shl::<u64>("), "{rust}");
    assert!(rust.contains("nikaia_std::proven::sub::<u32>("), "{rust}");
    prints("proved-shift", source, "16777216 0 768\n");
}

/// ADR-306 D18: a count nothing bounds keeps its check at every level, and a
/// shift by 64 stops the program.
#[test]
fn a_shift_by_an_unbounded_count_keeps_its_check() {
    let source = "\
fn down(y: u64, k: i64) -> u64 {
    return y >> k
}

fn main() {
    println(f\"{down(256, 4)}\")
    println(f\"{down(1, 64)}\")
}
";
    assert_eq!(unchecked_arithmetic(&lowered(source, Level::Aggressive)), 0);
    stops("unbounded-shift", source, "shift right with overflow");
}

const TALLY: &str = "\
fn main() {
    let mut seen: Vec[i64] = []
    for i in 0..<100 {
        seen.push(i % 10)
    }
    let mut counts: Vec[i64] = []
    counts.resize(10, 0)
    let mut total = 0
    for k in 0..<seen.len() {
        total += counts[seen[k]] + seen[k]
    }
    println(f\"{total}\")
}
";

/// D2, D3: every value pushed is `i % 10`, so a value read back indexes a
/// list of ten.
#[test]
fn a_value_read_out_of_a_bounded_list_indexes_without_its_check() {
    let off = unchecked_indexes(&lowered(TALLY, Level::Off));
    let on = unchecked_indexes(&lowered(TALLY, Level::Aggressive));
    assert_eq!(off, 0);
    // `seen[k]` twice by its own length, `counts[seen[k]]` by `seen`'s values.
    assert_eq!(on, 3, "{}", lowered(TALLY, Level::Aggressive));
    prints("tally", TALLY, "450\n");
}

/// **A bound as wide as `i64` is not a fact** (#444): `c` below is bound
/// only by its type, and stating `c >= -2^63` beside the others made the
/// solver refuse the whole query, so `xs[b]` after it kept its check.
#[test]
fn a_bound_as_wide_as_the_type_leaves_the_other_facts_usable() {
    let source = "\
fn pick(xs: ref Vec[i64], k: i64, h: u64) -> i64 {
    assert(xs.len() == 16)
    let b = k & 15
    let c = (h >> 1).bits_as_i64()
    return xs[b] + c
}

fn main() {
    let mut xs: Vec[i64] = []
    xs.resize(16, 2)
    println(f\"{pick(xs, 21, 6)}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert!(rust.contains("proven::read(&xs"), "{rust}");
    prints("wide-bound", source, "5\n");
}

/// **A `u64` has no upper bound the walk can state**: `bits_as_u64()` of
/// `-1` is `2^64 - 1`, so shifted by 56 it is 255. Read as at most
/// `i64::MAX`, it was "at most 127" and the index below was written without
/// its check, reading past a list of 128.
#[test]
fn a_u64_read_as_bits_is_not_bounded_by_i64_max() {
    let source = "\
fn pick(xs: ref Vec[i64], k: i64) -> i64 {
    assert(xs.len() == 128)
    let u = (k.bits_as_u64() >> 56).bits_as_i64()
    return xs[u]
}

fn main() {
    let mut xs: Vec[i64] = []
    xs.resize(128, 1)
    println(f\"{pick(xs, -1)}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert!(!rust.contains("proven::read(&xs"), "{rust}");
    stops("u64-bits", source, "index");
}

/// **Every write counts**: one value past the bound, written after the loop,
/// and the read is not proved - it stops at the index.
#[test]
fn a_later_write_past_the_bound_keeps_the_check() {
    let source = "\
fn main() {
    let mut seen: Vec[i64] = []
    for i in 0..<5 {
        seen.push(i % 10)
    }
    seen.push(10)
    let mut counts: Vec[i64] = []
    counts.resize(10, 0)
    let mut total = 0
    for k in 0..<seen.len() {
        total += counts[seen[k]]
    }
    println(f\"{total}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert!(!rust.contains("proven::read(&counts"), "{rust}");
    stops("later-write", source, "index");
}

/// A write the walk does not see - in a function with a `mut` parameter -
/// leaves the list without a bound.
#[test]
fn a_list_handed_to_a_function_that_changes_it_has_no_bound() {
    let source = "\
fn spoil(mut xs: Vec[i64]) {
    xs.push(99)
}

fn main() {
    let mut seen: Vec[i64] = []
    for i in 0..<5 {
        seen.push(i % 3)
    }
    spoil(seen)
    let mut counts: Vec[i64] = []
    counts.resize(3, 0)
    let mut total = 0
    for k in 0..<seen.len() {
        total += counts[seen[k]]
    }
    println(f\"{total}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert!(!rust.contains("proven::read(&counts"), "{rust}");
    stops("handed-on", source, "index");
}

/// **A bound may not hold by assuming itself**: each value is the last plus
/// one, which no bound found from an earlier pass covers.
#[test]
fn a_bound_that_only_holds_by_assuming_it_is_dropped() {
    let source = "\
fn main() {
    let mut xs: Vec[i64] = [0]
    for i in 0..<3 {
        xs.push(xs[i] + 1)
    }
    let small = [7, 8, 9]
    println(f\"{small[xs[3]]}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert!(!rust.contains("proven::read(&small"), "{rust}");
    stops("self-assumed", source, "index");
}

/// D4: one `push` per turn of `0..<n` makes the list `n` long.
#[test]
fn a_loop_that_pushes_once_per_turn_sets_the_length() {
    let source = "\
fn main() {
    let mut xs: Vec[i64] = []
    for i in 0..<10 {
        xs.push(i)
    }
    let mut total = 0
    for i in 0..<10 {
        total += xs[i]
    }
    println(f\"{total}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert!(rust.contains("proven::read(&xs"), "{rust}");
    prints("filled", source, "45\n");
}

/// D4 does not hold for a turn that can end before its `push`.
#[test]
fn a_loop_that_can_skip_its_push_says_nothing_of_the_length() {
    let source = "\
fn main() {
    let mut xs: Vec[i64] = []
    for i in 0..<10 {
        if i == 3 {
            continue
        }
        xs.push(i)
    }
    println(f\"{xs[9]}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert!(!rust.contains("proven::read(&xs"), "{rust}");
    stops("skipped-push", source, "index");
}

/// A value read out of a list bounds an operation on it: `250 + 10` does not
/// fit a `u8`, so the check stays and the program stops.
#[test]
fn a_bounded_value_too_large_for_the_operation_keeps_its_check() {
    let source = "\
fn main() {
    let mut xs: Vec[u8] = []
    xs.push(250)
    let y = xs[0] + 10
    println(f\"{y}\")
}
";
    assert_eq!(unchecked_arithmetic(&lowered(source, Level::Aggressive)), 0);
    stops("too-large", source, "overflow");
}

/// A list a lambda writes has no bound: the lambda may run at any time.
#[test]
fn a_list_a_lambda_writes_has_no_bound() {
    let source = "\
fn main() {
    let mut xs: Vec[i64] = [1]
    let grow = fn (v) { xs.push(v) }
    grow(5)
    let small = [7, 8]
    println(f\"{small[xs[1]]}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert!(!rust.contains("proven::read(&small"), "{rust}");
}

/// D1: the words.
#[test]
fn the_overflow_option_is_read_and_a_wrong_level_is_refused() {
    use nikaia::project::optimizations;
    let (bounds, overflow) =
        optimizations("remove-bounds-checks:off,remove-overflow-checks:on").unwrap();
    assert_eq!(bounds, BoundsChecks::Kept);
    assert_eq!(overflow, OverflowChecks::Removed);
    assert_eq!(optimizations("").unwrap().1, OverflowChecks::Kept);
    let basic = optimizations("remove-overflow-checks:basic").unwrap_err();
    assert!(basic.to_string().contains("`on` or `off`"), "{basic}");
}

/// **A compound assignment is proved as the operation it is** (#387,
/// ADR-306 D5a): `other += 7` after `other = v`, with `v < 2000`, and then
/// `other *= 2`; one that may overflow keeps its check.
#[test]
fn a_compound_assignment_proved_inside_its_type_is_written_without_its_check() {
    let source = "\
fn main() {
    let n: u32 = 2000
    let mut other: u32 = 0
    for v in 0..<n {
        other = v
        other += 7
        other *= 2
    }
    println(f\"{other}\")
}
";
    assert_eq!(unchecked_arithmetic(&lowered(source, Level::Off)), 0);
    let rust = lowered(source, Level::Aggressive);
    assert!(
        rust.contains("other = unsafe { nikaia_std::proven::add::<u32>(other, 7)"),
        "{rust}"
    );
    assert!(
        rust.contains("other = unsafe { nikaia_std::proven::mul::<u32>(other, 2)"),
        "{rust}"
    );
    prints("proved-compound", source, "4012\n");

    let wide = "\
fn main() {
    let n: u32 = 4000000000
    let mut other: u32 = 0
    for v in 0..<n {
        other = v
        other += 400000000
        if v == 3 {
            break
        }
    }
    println(f\"{other}\")
}
";
    let rust = lowered(wide, Level::Aggressive);
    assert_eq!(unchecked_arithmetic(&rust), 0, "{rust}");
    assert!(rust.contains("other += 400000000"), "{rust}");
}

/// **`xs[i] += d` at a proved position reads the slot once and writes it
/// back** (#387, ADR-306 D5a); a list of text is changed through the
/// proved place, without its check either (ADR-306 D5).
#[test]
fn a_compound_assignment_at_a_proved_position_is_read_and_written_unchecked() {
    let source = "\
fn bump(mut xs: Vec[i64]) {
    for i in 0..<xs.len() {
        xs[i] *= 3
    }
}

fn main() {
    let mut xs: Vec[i64] = [1, 2, 3]
    let mut names: Vec[String] = [\"a\", \"b\", \"c\"]
    for i in 0..<xs.len() {
        xs[i] += 10
        names[i] += \"!\"
    }
    bump(xs)
    println(f\"{xs[0]} {xs[2]} {names[1]}\")
}
";
    assert_eq!(unchecked_indexes(&lowered(source, Level::Off)), 0);
    let rust = lowered(source, Level::Aggressive);
    assert!(
        rust.contains("let __nikaia_stored = *unsafe { nikaia_std::proven::read(&xs, __nikaia_at) } + __nikaia_stored"),
        "{rust}"
    );
    assert!(
        rust.contains("nikaia_std::proven::write(&mut *xs, __nikaia_at, __nikaia_stored)"),
        "{rust}"
    );
    assert!(
        rust.contains("(*unsafe { nikaia_std::proven::slot(&mut names, (i) as usize) }) += "),
        "{rust}"
    );
    prints("proved-compound-index", source, "33 39 b!\n");
}

/// **A length whose elements take space is below 2^60** (#383, ADR-306 D12),
/// and an `i64` goal is stated inside ±2^62 so the solver can decide it
/// (#444): `x.len() + 1`, a sum and a difference of lengths are proved; a
/// product of two is not, and neither is the length of a list of a zero-sized
/// type.
#[test]
fn a_length_is_below_two_to_the_sixty_where_its_elements_take_space() {
    let source = "\
enum One {
    Only,
}

fn after(xs: ref Vec[i64]) -> i64 {
    return xs.len() + 1
}

fn both(a: ref Vec[i64], b: ref String) -> i64 {
    return a.len() + b.len()
}

fn inner(c: ref Vec[i64], w: ref Vec[i64]) -> i64 {
    return c.len() - w.len() - 1
}

fn grid(a: ref Vec[i64], b: ref Vec[i64]) -> i64 {
    return (a.len() + 1) * (b.len() + 1)
}

fn ones(xs: ref Vec[One]) -> i64 {
    return xs.len() + 1
}

fn main() {
    let xs: Vec[i64] = [1, 2]
    let os: Vec[One] = [One::Only]
    println(f\"{after(xs)} {both(xs, \"ab\")} {inner(xs, xs)} {grid(xs, xs)} {ones(os)}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    let body = |name: &str| {
        let at = rust
            .find(&format!("fn {name}("))
            .expect("the function is lowered");
        rust[at..].lines().next().unwrap_or_default().to_string()
    };
    assert!(
        body("after").contains("nikaia_std::proven::add::<i64>("),
        "{rust}"
    );
    assert!(
        body("both").contains("nikaia_std::proven::add::<i64>("),
        "{rust}"
    );
    assert!(
        body("inner").contains("nikaia_std::proven::sub::<i64>("),
        "{rust}"
    );
    assert!(!body("grid").contains("proven::mul"), "{rust}");
    assert_eq!(unchecked_arithmetic(&body("ones")), 0, "{rust}");
    prints("lengths", source, "3 4 -1 9 2\n");
}

/// **A list of a zero-sized type is bounded by how it was built** (#383):
/// `resize(n, v)` with a bounded `n`, and `extend` adding another's length.
#[test]
fn a_zero_sized_list_is_bounded_by_how_it_was_built() {
    let source = "\
enum One {
    Only,
}

fn main() {
    let n: i64 = 1000
    let mut c: Vec[One] = []
    c.resize(n, One::Only)
    let z = c.len() + 1
    let mut d: Vec[One] = []
    d.extend(c)
    let w = d.len() + 1
    println(f\"{z} {w}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert_eq!(unchecked_arithmetic(&rust), 2, "{rust}");
    prints("zero-sized", source, "1001 1001\n");
}

/// #699: `i + k` and `i += 1` inside `while i < b.len()`: the condition holds
/// in the body (ADR-306 D4), and a constant that is not negative moves `i`
/// up only, so the bound below is not asked (D6).
#[test]
fn a_step_under_a_while_over_a_length_is_proved() {
    let typed = "\
fn a(b: Vec[u8]) -> i64 {
    let mut i: i64 = 0
    let mut s: i64 = 0
    while i < b.len() { s = i + 8; i += 1 }
    return s
}

fn main() {
    println(f\"{a([1, 2, 3])}\")
}
";
    let rust = lowered(typed, Level::Aggressive);
    assert_eq!(unchecked_arithmetic(&rust), 2, "{rust}");
    prints("while-step", typed, "10\n");
    // An untyped `let mut i = 0` is a site as a typed one is.
    let untyped = typed.replace("let mut i: i64 = 0", "let mut i = 0");
    let rust = lowered(&untyped, Level::Aggressive);
    assert_eq!(unchecked_arithmetic(&rust), 2, "{rust}");
    assert!(rust.contains("nikaia_std::proven::add::<i64>("), "{rust}");
    prints("while-step-untyped", &untyped, "10\n");
}

/// #699: a parameter under `if i < b.len()` is proved from the upper bound
/// alone; the same `i + 8` with nothing above it keeps its check.
#[test]
fn an_upper_bound_alone_proves_a_constant_step() {
    let guarded = "\
fn a(b: Vec[u8], i: i64) -> i64 {
    if i < b.len() { return i + 8 }
    return 0
}

fn main() {
    println(f\"{a([1, 2, 3], -5)}\")
}
";
    assert_eq!(
        unchecked_arithmetic(&lowered(guarded, Level::Aggressive)),
        1
    );
    prints("upper-bound-step", guarded, "3\n");
    let unguarded = "\
fn a(i: i64) -> i64 {
    return i + 8
}

fn main() {
    println(f\"{a(9223372036854775807)}\")
}
";
    assert_eq!(
        unchecked_arithmetic(&lowered(unguarded, Level::Aggressive)),
        0
    );
    stops("unguarded-step", unguarded, "overflow");
}

/// `i += d` on an untyped `let mut i = 0` takes `d`'s type and is proved as
/// `i + d` (ADR-285 D23, ADR-306 D9), also where `d` is a number this use
/// types; a `d` nothing bounds keeps its check.
#[test]
fn a_compound_step_on_an_untyped_name_is_proved() {
    let source = "\
fn a(d: i64, e: i64) -> i64 {
    let mut i = 0
    if d >= 0 && d < 4 {
        i += d
    }
    i += e
    return i
}

fn b(d: i64) -> i64 {
    let mut i = 0
    let k = if d > 2 { 3 } else { -1 }
    if k >= 0 {
        i += k
    }
    return i
}

fn main() {
    println(f\"{a(3, 1)} {b(3)}\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert_eq!(
        rust.matches("nikaia_std::proven::add::<i64>(i, ").count(),
        2,
        "{rust}"
    );
    assert!(rust.contains("i += e"), "{rust}");
    prints("untyped-compound", source, "4 3\n");
}

/// **A field of an element is a number of its own** (#781): an `assume` on
/// `self.slots[at].sum` proves `self.slots[at].sum += v`, also after a write
/// to another field of the same element. A write that may reach the same
/// field of the element - through another index, after the index moved, in
/// a branch, by a method, by a `push`, or to the whole element - takes what
/// was known of it.
#[test]
fn a_field_of_an_element_bounded_by_an_assume_is_proved() {
    let cases = [
        ("same_element_other_field", "self.slots[at].min = 5", true),
        ("nothing_between", "", true),
        (
            "other_index",
            "self.slots[j].sum = 9223372036854775807",
            false,
        ),
        ("index_moved", "at = j", false),
        (
            "in_a_branch",
            "if v > 3 { self.slots[at].sum = 9223372036854775807 }",
            false,
        ),
        (
            "other_index_in_a_branch",
            "if v > 3 { self.slots[j].sum = 9223372036854775807 }",
            false,
        ),
        ("index_moved_in_a_branch", "if v > 3 { at = j }", false),
        ("by_a_method", "self.bump()", false),
        (
            "by_a_push",
            "self.slots.push(Slot { min: 0, sum: 0 })",
            false,
        ),
        (
            "the_whole_element",
            "self.slots[j] = Slot { min: 0, sum: 9223372036854775807 }",
            false,
        ),
    ];
    let mut source = String::from(
        "struct Slot {\n    min: i64,\n    sum: i64,\n}\n\nstruct Table {\n    slots: Vec[Slot],\n}\n\n\
         impl Table {\n    fn bump(ref mut self) {\n        self.slots[0].sum = 9223372036854775807\n    }\n",
    );
    for (name, between, _) in &cases {
        source.push_str(&format!(
            "    fn {name}(ref mut self, first: i64, j: i64, v: i64) -> i64 {{\n\
             \x20       let mut at = first\n\
             \x20       unsafe {{\n\
             \x20           assume(at >= 0 && at < self.slots.len() && j >= 0 && j < self.slots.len())\n\
             \x20           assume(self.slots[at].sum >= -1000 && self.slots[at].sum <= 1000)\n\
             \x20           assume(v >= -999 && v <= 999)\n\
             \x20       }}\n\
             \x20       {between}\n\
             \x20       self.slots[at].sum += v\n\
             \x20       return self.slots[at].sum\n\
             \x20   }}\n"
        ));
    }
    source.push_str(
        "}\n\nfn main() {\n    let mut t = Table { slots: [] }\n    \
         t.slots.push(Slot { min: 0, sum: 7 })\n    \
         println(f\"{t.same_element_other_field(0, 0, 5)} {t.nothing_between(0, 0, -2)}\")\n}\n",
    );
    let rust = lowered(&source, Level::Aggressive);
    for (name, _, proved) in &cases {
        let body = rust
            .split(&format!("fn {name}("))
            .nth(1)
            .and_then(|rest| rest.split("\n    }").next())
            .unwrap_or_else(|| panic!("{name} is emitted: {rust}"));
        assert_eq!(
            body.contains(".sum = unsafe { nikaia_std::proven::add::<i64>("),
            *proved,
            "{name}: {body}"
        );
    }
    prints("element-field", &source, "12 10\n");
}

/// `x ?? d` lies where the number in `x` and `d` do, with what the ledger
/// ensures of a callee's `result` said of the number in it: a mask's first
/// lane is below 64 (ADR-377 D7), so it indexes 64 slots without a check.
#[test]
fn what_a_callee_ensures_holds_of_the_number_an_option_gives() {
    let source = "\
use std::simd

fn a(b: ref Bytes, xs: ref Vec[i64]) -> i64 {
    assert(b.len() >= 32 && xs.len() == 64)
    let v: simd::Fixed[u8, 32] = simd::load(b, 0)
    let s: simd::Fixed[u8, 32] = simd::splat(';')
    let first = v.eq(s).first() ?? 0
    let other = v.eq(s).first() ?? -1
    let mut r = xs[first]
    if other >= 0 { r = r + xs[other] }
    return r
}

fn main() {
    println(\"\")
}
";
    let rust = lowered(source, Level::Aggressive);
    assert!(
        rust.contains("nikaia_std::proven::read(&xs, (first) as usize)"),
        "{rust}"
    );
    assert!(
        rust.contains("nikaia_std::proven::read(&xs, (other) as usize)"),
        "{rust}"
    );
    // Past the slots by up to 63: the index keeps its check.
    let past = source.replace(
        "let other = v.eq(s).first() ?? -1",
        "let other = xs.len() - 1 + (v.eq(s).first() ?? 0)",
    );
    let rust = lowered(&past, Level::Aggressive);
    assert!(
        !rust.contains("proven::read(&xs, (other) as usize)"),
        "{rust}"
    );
}
