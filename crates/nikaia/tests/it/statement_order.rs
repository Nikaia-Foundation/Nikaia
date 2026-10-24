//! **An unconditional write is emitted before the `if`s in front of it that
//! cannot tell** ([ADR-379](../../../docs/specification/adr/adr-379.md)).
//!
//! Each case is one method: two `if`s on a slot, then the write the rule may
//! move. The write moves only where nothing it touches is touched by the
//! `if`s and neither side can stop the program (D1), and in every build the
//! same (D2): each case is lowered with the switches off and on.

use crate::common;
use std::process::Command;

use nikaia::bounds::{BoundsChecks, OverflowChecks};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn builds() -> [Build; 2] {
    [
        Build::default(),
        Build {
            bounds: BoundsChecks::Removed,
            overflow: OverflowChecks::Removed,
            ..Build::default()
        },
    ]
}

/// What a case says about `at`, `v` and the slot's sum, unless it leaves one
/// out to keep a check unproved.
const ASSUMED: &str = "assume(at >= 0 && at < self.slots.len())\n            \
                       assume(self.slots[at].sum >= -1000 && self.slots[at].sum <= 1000)\n            \
                       assume(self.slots[at].count >= 0 && self.slots[at].count <= 1000)\n            \
                       assume(v >= -999 && v <= 999)";

/// The cases: a name, what is assumed, the two `if`s, the writes after them,
/// and whether the first of those writes is emitted before the `if`s.
const CASES: [(&str, &str, &str, &str, bool); 7] = [
    (
        "moves",
        ASSUMED,
        "if v < self.slots[at].min { self.slots[at].min = v }\n        \
         if v > self.slots[at].max { self.slots[at].max = v }",
        "self.slots[at].sum += v\n        self.slots[at].count += 1",
        true,
    ),
    (
        "an_if_reads_it",
        ASSUMED,
        "if v < self.slots[at].sum { self.slots[at].min = v }\n        \
         if v > self.slots[at].max { self.slots[at].max = v }",
        "self.slots[at].sum += v",
        false,
    ),
    (
        "an_if_writes_what_it_reads",
        ASSUMED,
        "if v < self.slots[at].min { self.slots[at].max = 1 }\n        \
         if v > self.slots[at].min { self.slots[at].min = v }",
        "self.slots[at].count = self.slots[at].min",
        false,
    ),
    (
        "an_if_calls_out",
        ASSUMED,
        "if v < self.slots[at].min { println(\"low\") }\n        \
         if v > self.slots[at].max { self.slots[at].max = v }",
        "self.slots[at].sum += v",
        false,
    ),
    (
        "its_overflow_is_not_proved",
        "assume(at >= 0 && at < self.slots.len())\n            assume(v >= -999 && v <= 999)",
        "if v < self.slots[at].min { self.slots[at].min = v }\n        \
         if v > self.slots[at].max { self.slots[at].max = v }",
        "self.slots[at].sum += v",
        false,
    ),
    (
        "an_index_is_not_proved",
        "assume(v >= -999 && v <= 999)",
        "if v < self.slots[0].min { self.slots[0].min = v }\n        \
         if v > self.slots[at].max { self.slots[at].max = v }",
        "self.used += 1",
        false,
    ),
    (
        "an_if_leaves",
        ASSUMED,
        "if v < self.slots[at].min { return 0 }\n        \
         if v > self.slots[at].max { self.slots[at].max = v }",
        "self.slots[at].sum += v",
        false,
    ),
];

fn source() -> String {
    let mut source = String::from(
        "struct Slot {\n    min: i64,\n    max: i64,\n    sum: i64,\n    count: i64,\n}\n\n\
         struct Table {\n    slots: Vec[Slot],\n    used: i64,\n}\n\nimpl Table {\n",
    );
    for (name, assumed, ifs, writes, _) in CASES {
        source.push_str(&format!(
            "    fn {name}(ref mut self, at: i64, v: i64) -> i64 {{\n        \
             unsafe {{\n            {assumed}\n        }}\n        \
             {ifs}\n        {writes}\n        \
             return self.slots[0].sum\n    }}\n"
        ));
    }
    source.push_str(
        "}\n\nfn main() {\n    let mut t = Table { slots: [], used: 0 }\n    \
         t.slots.push(Slot { min: 0, max: 0, sum: 0, count: 0 })\n",
    );
    for (name, ..) in CASES {
        source.push_str(&format!("    println(f\"{{t.{name}(0, 5)}}\")\n"));
    }
    source.push_str("}\n");
    source
}

/// The emitted body of method `name`.
fn body<'r>(rust: &'r str, name: &str) -> &'r str {
    rust.split(&format!("fn {name}("))
        .nth(1)
        .and_then(|rest| rest.split("\n    }").next())
        .unwrap_or_else(|| panic!("{name} is emitted:\n{rust}"))
}

/// Where the first write to the field `field` stands in `body`: `x.f = …`
/// where its check is dropped, `x.f += …` where it is kept.
fn write_of(body: &str, field: &str) -> Option<usize> {
    [format!(".{field} = "), format!(".{field} += ")]
        .iter()
        .filter_map(|w| body.find(w.as_str()))
        .min()
}

/// D1, D2: the write moves where the rule allows it, and nowhere else, in a
/// build with the switches off as with them on.
#[test]
fn an_unconditional_write_moves_before_the_ifs_only_where_nothing_can_tell() {
    let source = source();
    let parsed = parse_to_ast(&source).expect("the source parses");
    for build in builds() {
        let removed = build.bounds == BoundsChecks::Removed;
        let rust = emit_program(&parsed, build).expect("it lowers").rust;
        for (name, _, _, writes, moves) in CASES {
            let body = body(&rust, name);
            let first_if = body.find("if v ").expect("an if");
            let target = writes.split(' ').next().expect("a target");
            let written = target.rsplit('.').next().expect("a field");
            let write = write_of(body, written)
                .unwrap_or_else(|| panic!("{name}: the write to {written}:\n{body}"));
            assert_eq!(
                write < first_if,
                moves,
                "{name}, switches removed: {removed}:\n{body}"
            );
        }
        // The writes keep their own order: `sum` before `count`.
        let moved = body(&rust, "moves");
        assert!(
            write_of(moved, "sum").expect("sum") < write_of(moved, "count").expect("count"),
            "{moved}"
        );
    }
}

/// The program prints the same in both builds: the order changes nothing a
/// program can see.
#[test]
fn a_moved_write_changes_nothing_the_program_prints() {
    let source = source();
    let parsed = parse_to_ast(&source).expect("the source parses");
    let mut printed: Vec<String> = Vec::new();
    for (k, build) in builds().into_iter().enumerate() {
        let rust = emit_program(&parsed, build).expect("it lowers").rust;
        let dir = common::scratch_dir(&format!("statement-order-{k}"));
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
            "{}\n--- emitted ---\n{rust}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let out = Command::new(&binary).output().expect("run it");
        std::fs::remove_dir_all(&dir).ok();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        printed.push(String::from_utf8_lossy(&out.stdout).to_string());
    }
    assert_eq!(printed[0], printed[1]);
    assert_eq!(printed[0], "5\n10\n10\n15\n20\n20\n25\n");
}
