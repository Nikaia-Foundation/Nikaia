# The checks at scale: the compiler's own package (#1625)

How often the checker speaks on a large correct program, and how often it is
wrong when it does. The program is the self-hosted compiler, the tools
package `crates/nikaia-std/src/compiler/*.nika`. Each file is checked with
the rest of the package beside it, as `lower-std` checks it.

## Commands

    target/fast/nikaia lower-std --findings > REPORT     # about 2 minutes
    target/fast/nikaia run scripts/analysis_scale.nika -- REPORT [--seed N] [--per-code N]

`--findings` prints every warning the checker gives each file. A plain
`lower-std` drops warnings and stops only on a refusal. Without `REPORT` the
script runs `lower-std --findings` itself. The sample is seeded (default
1625, up to 20 sites per code), so the same sites come back on every run as
long as the code at them stays.

## One run

The run was made on develop `0305ee33` plus the `--findings` flag of this
change, with `target/fast/nikaia`.

* 271 files, 195,939 lines checked.
* 0 refused.
* 3,569 warnings, 18.21 per 1000 lines.

| Code | Warnings | Files | Per 1000 lines | What it says |
| :--- | ---: | ---: | ---: | :--- |
| NK1261 | 2,828 | 232 | 14.43 | `ref` on a parameter is redundant |
| NK1350 | 722 | 138 | 3.68 | a `.clone()` makes nothing new |
| NK1111 | 12 | 7 | 0.06 | braces in a plain string (migration notice) |
| NK1216 | 6 | 5 | 0.03 | `??` over a value that always has one |
| NK1217 | 1 | 1 | 0.01 | `?.` on a value that is never absent |

Files with the most warnings:

| File | Warnings |
| :--- | ---: |
| `text_tiers.nika` | 150 |
| `bounds_walk.nika` | 111 |
| `bounds_rules.nika` | 94 |
| `view_changes.nika` | 87 |
| `sharing.nika` | 72 |

## Reviewed sample (seed 1625)

Each site was read in its source.

* **Correct**: the warning says something true about the code.
* **False alarm**: the code means what it says, and the warning should not
  be there.

| Code | Sample | Correct | False alarm | Unsure | False-alarm rate |
| :--- | ---: | ---: | ---: | ---: | ---: |
| NK1261 | 20 of 2,828 | 20 | 0 | 0 | 0 % (n = 20) |
| NK1350 | 20 of 722 | 20 | 0 | 0 | 0 % (n = 20) |
| NK1111 | 12 of 12 | 3 | 9 | 0 | 75 % (n = 12), all by design |
| NK1216 | 6 of 6 | 6 | 0 | 0 | 0 % (n = 6) |
| NK1217 | 1 of 1 | 1 | 0 | 0 | 0 % (n = 1) |

**NK1261.** All 20 sites are parameters written `ref String`, `ref String?`
or `ref collections::BTreeMap[..]`, where a parameter already lends.

* At `emit_core_calls.nika:2056` two sites share a line (`owner` and
  `field`).

**NK1350.** All 20 sites are correct:

* 13 copy a value that copies itself: `Range` in `bounds_walk.nika` (8),
  `EmitLifetimes` in `emit_item.nika` (5).
* 1 more copies itself too: a `winnow_grammar::Symbol`
  (`emit_stmt.nika:1294`).
* 4 are the last use of an owned local (`bounds_shape.nika:157`,
  `emit_stmt.nika:1169` and `:1285`, `parse_errors.nika:254`).
* 2 copy a part of a parameter that is only read (`parse_errors.nika:136`
  and `:391`).

**NK1216 and NK1217.** Each site sits where the value is known to be there:

* behind an `abi != null`;
* in the arm after a `null =>` arm (`grammar_types.nika:720`,
  `view_changes.nika:815` and `:1277`);
* on a `Range` result (`bounds_walk.nika:1990`);
* on a `Vec[String]` field read with `?.` (`emit_core_reads.nika:315`).

**NK1111.** Three sites are correct, and each is a real defect in the
compiler:

* `check_host.nika:516`: NK1111's own headline read `` `{{{{` `` where it
  means `` `{{` ``. Fixed in this change.
* `emit_item.nika:486`: NK1227's fallback message said
  `` write `extern {{ … }}` ``. Fixed in this change.
* `sysroot_package.nika:308`: the lowered `use ...::{{CopyOwned, ..}}` is
  a stale escape. It works only because Rust reads `{{..}}` as a nested use
  group.

The other nine are migration notices on text that means its braces
literally. They are by design: `unmarked_hole` warns on every doubled brace,
and on every hole that names something here, because a migration notice
cannot know intent. The sites:

* the f-string escape handling of `emit_core_leaves.nika:286` and `:411`,
  two strings on each line;
* the Rust format text of `emit_text.nika:62` and `:64`;
* the Rust code text `-> { Vec::new() }` of `emit_grammar.nika:565` and
  `:1013`;
* the message text `` `unsafe { assume(cond) }` `` of
  `check_core_expr.nika:597`.

So a false alarm here is no bug to file. It counts against keeping the
notice once the migration is over.

**Site lines.** NK1111 and NK1216 inside a `match` that is bound by a
`let` point at the `let`, not at the string or the `??`.
`sysroot_package.nika:308` and `view_changes.nika:815` are such sites;
the code they mean is two and nine lines further down. This was filed as
its own defect.

## Cost per analysis

`lower-std --findings` took 2 min 0.7 s wall time (2 min 30.9 s user) for
the 271 files. Instructions per analysis cannot be read today.

* `NIKAIA_LOWER_TIMES=1` gives wall time only:
  * per `lower-std` phase: parse, ledger, trust, graphs, names, files,
    assemble;
  * per ledger pass (`InferHost::timed` in `contracts/mod.rs`): check_units,
    sync, throws, touch, sharing, maps, locks, panics, tether, ...;
  * per file, the check and emit time summed.
* The checker's own analyses run inside the one `check` call per file, with
  no split. These are handles (ADR-312), outside data (ADR-396), bounds
  (ADR-385), the clone and `ref` lints, and the rest.

A report of instructions per analysis is its own work (the issue's item 3).
