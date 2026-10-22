//! **A callee's contract is a fact at its call** (#776): what a method's
//! postconditions say of its result proves an index after the call, a mask's
//! or a shift's value is a bounded whole number to the prover, a loop that
//! gives a name only such values keeps it bounded, and `panic(…)` is no exit.
//! A signed value shifted right by all but one of its bits is `0` or `-1`, and
//! `^` of two values inside `-2^m..<2^m` stays inside it.

mod common;

use std::path::Path;
use std::process::{Command, Output};

const ALL_ON: &str = "remove-bounds-checks:on,remove-overflow-checks:on,prove-through-loops:on";

fn nikaia(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("the nikaia binary runs")
}

/// The lowered Rust of `source` with every optimization on, and the ledger
/// written beside it.
fn lowered(purpose: &str, source: &str) -> (String, String) {
    let dir = common::scratch_dir(purpose);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let out = nikaia(&dir, &["lower", "main.nika", "--optimization", ALL_ON]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rust = std::fs::read_to_string(dir.join("main.rs")).expect("the lowered program");
    let ledger = std::fs::read_to_string(dir.join("nikaia.contracts")).expect("the ledger");
    let _ = std::fs::remove_dir_all(&dir);
    (rust, ledger)
}

/// The body of the lowered function `name`, up to its end.
fn body<'a>(rust: &'a str, name: &str) -> &'a str {
    let at = rust
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("{name}: {rust}"));
    let rest = &rust[at..];
    &rest[..rest
        .find("\n    }\n")
        .or(rest.find("\n}"))
        .expect("its end")]
}

/// A table of a type with a rule, its slot found by a method through a loop
/// that ends in `panic`: the method's postcondition is published, and the
/// index after the call is written without its check.
const TABLE: &str = "struct Slot {
    key: i64,
    count: i64,
}

struct Table {
    slots: Vec[Slot],
    assert(slots.len() == 16384)
}

impl Table {
    fn slot_of(ref mut self, key: i64, h: u64) -> i64 {
        let mut at = (h >> 50).bits_as_i64()
        while true {
            if self.slots[at].count == 0 || self.slots[at].key == key {
                assert(at >= 0 && at < 16384)
                return at
            }
            at = (at + 1) & 16383
        }
        panic(\"the table is full\")
    }

    fn add(ref mut self, key: i64, h: u64) {
        let at = self.slot_of(key, h)
        self.slots[at].key = key
        self.slots[at].count = 1
    }
}

fn main() {
    let mut slots: Vec[Slot] = []
    slots.resize(16384, Slot { key: 0, count: 0 })
    let mut t = Table { slots: slots }
    t.add(7, 0x9E3779B97F4A7C15)
    t.add(7, 0x9E3779B97F4A7C15)
    println(f\"{t.slot_of(7, 0x9E3779B97F4A7C15)}\")
}
";

#[test]
fn a_methods_postcondition_proves_the_index_after_its_call() {
    let (rust, ledger) = lowered("facts-at-calls-method", TABLE);
    let entry = &ledger[ledger.find("[fn.\"Table::slot_of\"]").expect("its entry")..];
    let entry = &entry[..entry[1..].find("\n[").map_or(entry.len(), |e| e + 1)];
    assert!(
        entry.contains("ensures = [\"result >= 0 && result < 16384\"]"),
        "{entry}"
    );
    let add = body(&rust, "add");
    assert!(!add.contains("index::get"), "{add}");
}

/// A signed value shifted right by 63 is `0` or `-1`; `a ^ s` of a value in
/// `0..=1023` with it is inside `-1024..=1023`, and `- s` fits.
#[test]
fn a_sign_mask_and_its_xor_stay_inside_their_type() {
    let source = "fn tenths(w: u64, a: u64) -> i64 {
    let s = (w << 59).bits_as_i64() >> 63
    let abs = (a & 0x3FF).bits_as_i64()
    return (abs ^ s) - s
}

fn main() {
    println(f\"{tenths(0, 5)} {tenths(16, 5)}\")
}
";
    let (rust, _) = lowered("facts-at-calls-sign", source);
    let tenths = body(&rust, "tenths");
    assert!(tenths.contains("proven::sub::<"), "{tenths}");
}

/// **A method's precondition is a goal at `self.m(…)`** (#776): proved there,
/// also inside the block that checks the type's rule around a `ref mut self`
/// call, so the call reaches the entry that does not check it; a call that
/// does not prove it reaches the one that does.
const SELF_CALL: &str = "struct T {
    n: i64,
    assert(n >= 0)
}

impl T {
    fn get(ref mut self, xs: ref Vec[i64], i: i64) -> i64 {
        assert(i >= 0 && i < xs.len())
        self.n = 1
        return xs[i] + self.n
    }

    fn sum(ref mut self, xs: ref Vec[i64]) -> i64 {
        let mut s = 0
        for i in 0..<xs.len() {
            let v = self.get(xs, i)
            s = s.wrapping_add(v)
        }
        return s
    }

    fn past(ref mut self, xs: ref Vec[i64]) -> i64 {
        let mut s = 0
        for i in 0..<xs.len() {
            s = s.wrapping_add(self.get(xs, i + 1))
        }
        return s
    }
}

fn main() {
    let mut t = T { n: 1 }
    println(f\"{t.sum([1, 2, 3])}\")
}
";

#[test]
fn a_methods_precondition_is_proved_at_a_call_on_self() {
    let (rust, _) = lowered("facts-at-calls-self-pre", SELF_CALL);
    let sum = body(&rust, "sum");
    assert!(sum.contains("self.get__unchecked(xs, i)"), "{sum}");
    let past = body(&rust, "past");
    assert!(
        past.contains("self.get(") && !past.contains("get__unchecked"),
        "{past}"
    );
}

/// **The bounds walk proves a method's precondition where the prover reads
/// too little** (#776): a position `find_all` yields and a length cut from it
/// in a loop that stops before `stop`, as `benches/brc/fast.nika`'s does,
/// and a receiver other than `self`.
const WALKED_CALL: &str = "struct T {
    n: i64,
}

impl T {
    fn name(ref self, buf: ref Bytes, pos: i64, len: i64) -> i64 {
        assert(pos >= 0 && len >= 0 && pos + len <= buf.len())
        return buf[pos..<pos + len].len() + self.n
    }

    fn names(ref self, buf: ref Bytes, stop: i64) -> i64 {
        assert(stop > 0 && stop <= buf.len() - 16)
        let mut pos = 0
        let mut total = 0
        for sep in buf.find_all(bytes::ByteSet([';'])) {
            let len = sep - pos
            assert(len >= 0 && len <= 100)
            total = total.wrapping_add(self.name(buf, pos, len))
            pos = sep + 1
            if pos >= stop { break }
        }
        return total
    }
}

fn main() {
    let t = T { n: 0 }
    let b = Bytes([97, 59, 98, 98, 59])
    if b.len() > 20 {
        println(f\"{t.names(b, 1)}\")
    }
    if b.len() > 2 {
        println(f\"{t.name(b, 0, 2)}\")
    }
}
";

#[test]
fn the_bounds_walk_proves_a_methods_precondition_at_its_call() {
    let source = format!("use std::bytes\n{WALKED_CALL}");
    let (rust, _) = lowered("facts-at-calls-walked-pre", &source);
    let names = body(&rust, "names__unchecked");
    assert!(names.contains("self.name__unchecked("), "{names}");
    assert!(rust.contains("t.name__unchecked(&b, 0, 2)"), "{rust}");
}

/// **Bytes equal a cut of bytes with the same content** (#776), either way
/// round: the comparison the checker accepts is one Rust accepts.
#[test]
fn bytes_compare_with_a_cut_of_bytes() {
    let dir = common::scratch_dir("facts-at-calls-bytes-cut");
    std::fs::write(
        dir.join("main.nika"),
        "fn main() {
    let a = Bytes([1, 2, 3])
    let b = Bytes([0, 1, 2, 3, 4])
    println(f\"{a == b[1..<4]} {b[1..<4] == a} {a == b[0..<3]}\")
}
",
    )
    .expect("the source");
    let out = nikaia(&dir, &["run", "main.nika"]);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "true true false"
    );
}
