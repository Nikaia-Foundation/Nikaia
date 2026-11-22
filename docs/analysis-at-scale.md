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
the 271 files; on 2026-11-21 (f03819a5e) 3 min 52.9 s wall, 6 min 22.4 s
user.

* `NIKAIA_LOWER_TIMES=1` gives wall time only:
  * per `lower-std` phase: parse, ledger, trust, graphs, names, files,
    assemble;
  * per ledger pass (`InferHost::timed` in `contracts/mod.rs`): check_units,
    sync, throws, touch, sharing, maps, locks, panics, tether, ...;
  * per file, the check and emit time summed.
* Instructions per analysis (#2187): `nikaia run scripts/analysis_cost.nika`
  runs `lower-std --findings` under callgrind, counting only the check walk
  of each file, and prints one row per analysis (`docs/measuring.md`,
  *Instructions per checker analysis*). Each analysis has its own
  `@inline(never)` functions in `.nika`: `check_walked_outside`,
  `check_walked_prove`, `check_walked_views`, `check_walked_unsigned_slots`;
  handles and the lints run woven into the type walk, so their rows count
  only the functions they call.

On one file, `nikaia check scripts/analysis_scale.nika` (179 lines,
2026-11-21, f03819a5e plus #2187's change, read with `--read`):

| Analysis | Instructions | Share of the walk |
| :--- | ---: | ---: |
| handles (ADR-312) | 2,502 | 0.0 % |
| outside data (ADR-396) | 10,176,230 | 8.2 % |
| bounds (ADR-385) | 0 | 0.0 % |
| proofs (ADR-269 D4, ADR-396 D5) | 43,878,584 | 35.4 % |
| views (NK2302, NK2305, NK2303) | 26,228,170 | 21.2 % |
| lints (NK1261, NK1350) | 189,707 | 0.2 % |
| rest of the walk | 43,371,707 | 35.0 % |
| the walk | 123,846,900 | 100.0 % |

Bounds reads 0 there: a plain `check` does not run the unsigned-slot pass.
The whole package was counted once, file by file, in the next section.

## Cost per file against its lines (#2649)

The package cannot be cut by its dependencies. 213 of its 271 files form one
cycle, 180,437 of about 205,000 lines (88 %), so no 25 %, 50 % or 75 % cut
keeps every file's dependencies. The unit is the file instead.

The package stays whole. Each file is checked with the others beside it, as
`lower-std` checks it. Only the counting is done per file.

The run:

* Command: `NIKAIA_LOWER_THREADS=1`, `nikaia run scripts/analysis_cost.nika --
  --files --out target/analysis-cost/files.callgrind`.
* Built from develop `9fe994126`. Run 2026-11-21 21:14 to 2026-11-22
  00:59 UTC.
* Read with `--files --read target/analysis-cost/files.callgrind --sample 24`.

The run holds 276 dumps:

* dump 1 is the ledger's walks of all 274 files (`infer_package_checked`),
  132,757,065,228 instructions, not split per file;
* dumps 2 to 275 are one per file, after each file's own check
  (`check_tools_file`, the check `lower-std --findings` reports). They hold
  305.1 G instructions over 207,743 lines;
* the last dump is the program's end, with nothing inside the walks.

**The sample** is `--sample 24` (`sample_ranks` in the script). The 274
files are sorted by lines. The sample takes the smallest file, the 5
largest, and 18 files at evenly spaced ranks in between. The quarters and
the fit below are over all 274 files.

| File | Lines | handles (ADR-312) | outside data (ADR-396) | bounds (ADR-385) | proofs (ADR-269 D4, ADR-396 D5) | views (NK2302, NK2305, NK2303) | lints (NK1261, NK1350) | rest of the walk | the walk | walk per line |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| sources.nika | 35 | 55 | 685,647 | 0 | 20,651,575 | 366,162,522 | 4,780 | 354,691,850 | 742,196,429 | 21,205,612 |
| prover_results.nika | 103 | 72 | 669,850 | 0 | 19,351,944 | 365,651,323 | 8,395 | 355,029,915 | 740,711,499 | 7,191,373 |
| interpreter.nika | 132 | 427 | 2,294,761 | 0 | 26,569,135 | 370,180,033 | 37,056 | 374,434,466 | 773,515,878 | 5,859,968 |
| check_literals.nika | 179 | 1,124 | 4,222,473 | 0 | 66,813,641 | 376,729,319 | 127,140 | 386,618,347 | 834,512,044 | 4,662,078 |
| comptime_driver.nika | 219 | 460 | 3,339,223 | 0 | 74,118,210 | 372,418,898 | 132,871 | 380,149,993 | 830,159,655 | 3,790,683 |
| emit_keeps.nika | 243 | 520 | 2,718,363 | 0 | 25,914,947 | 370,891,894 | 23,238 | 380,802,518 | 780,351,480 | 3,211,322 |
| fix.nika | 284 | 1,145 | 6,725,391 | 0 | 240,884,987 | 378,071,748 | 155,911 | 394,762,747 | 1,020,601,929 | 3,593,668 |
| check_notes.nika | 329 | 1,854 | 5,605,328 | 0 | 78,148,959 | 377,393,248 | 181,193 | 416,713,641 | 878,044,223 | 2,668,827 |
| emit_kinds.nika | 364 | 24 | 1,659,212 | 0 | 13,552,255 | 365,684,887 | 16,859 | 371,029,987 | 751,943,224 | 2,065,778 |
| check_lambdas.nika | 402 | 1,055 | 4,227,519 | 0 | 80,530,183 | 376,707,824 | 179,872 | 410,056,148 | 871,702,601 | 2,168,414 |
| emit_holes.nika | 463 | 928 | 5,982,299 | 0 | 106,945,469 | 386,093,109 | 222,398 | 421,971,264 | 921,215,467 | 1,989,666 |
| check_boundary.nika | 536 | 1,096 | 10,390,504 | 0 | 48,524,864 | 380,576,251 | 220,302 | 426,541,739 | 866,254,756 | 1,616,146 |
| project_check.nika | 613 | 1,834 | 13,489,361 | 0 | 118,218,667 | 386,365,976 | 161,192 | 477,808,557 | 996,045,587 | 1,624,870 |
| cexport_node_text.nika | 642 | 0 | 3,001,260 | 0 | 11,393,514 | 364,103,090 | 83,987 | 354,973,233 | 733,555,084 | 1,142,609 |
| emit_expr.nika | 803 | 4,232 | 16,976,335 | 0 | 353,528,919 | 407,419,015 | 346,116 | 501,817,216 | 1,280,091,833 | 1,594,136 |
| project_features.nika | 969 | 4,285 | 15,437,844 | 0 | 282,174,418 | 411,994,493 | 455,855 | 544,164,083 | 1,254,230,978 | 1,294,356 |
| check_state.nika | 1,083 | 213 | 3,449,072 | 0 | 25,044,068 | 368,569,984 | 88,784 | 413,149,009 | 810,301,130 | 748,200 |
| check_facts.nika | 1,394 | 134,329 | 47,907,034 | 0 | 1,276,260,233 | 428,450,922 | 975,232 | 614,571,109 | 2,368,298,859 | 1,698,923 |
| check_core_places.nika | 1,681 | 6,937 | 35,857,827 | 0 | 253,001,396 | 449,548,948 | 774,075 | 738,725,205 | 1,477,914,388 | 879,187 |
| views.nika | 3,912 | 497,014 | 86,771,955 | 0 | 1,080,050,503 | 593,395,107 | 2,002,551 | 1,173,152,701 | 2,935,869,831 | 750,477 |
| check_core_stmt.nika | 4,795 | 17,838 | 105,808,274 | 0 | 1,268,532,680 | 813,412,709 | 2,381,408 | 1,554,088,611 | 3,744,241,520 | 780,863 |
| prover_walk.nika | 5,605 | 12,768,128 | 138,014,783 | 0 | 1,423,964,262 | 806,168,477 | 4,951,417 | 2,266,262,780 | 4,652,129,847 | 829,996 |
| text_tiers.nika | 10,081 | 39,499 | 176,879,006 | 0 | 3,128,899,392 | 797,961,446 | 5,661,811 | 2,301,353,791 | 6,410,794,945 | 635,928 |
| bounds_walk.nika | 12,955 | 333,170 | 220,568,532 | 0 | 3,355,581,639 | 1,167,013,446 | 6,862,628 | 3,051,758,011 | 7,802,117,426 | 602,247 |

Instructions per line, all files sorted by lines and cut into quarters
(each quarter's instructions over its lines):

| Analysis | 35-241 lines | 242-439 lines | 441-896 lines | 924-12,955 lines |
| :--- | ---: | ---: | ---: | ---: |
| handles (ADR-312) | 328 | 208 | 94 | 180 |
| outside data (ADR-396) | 17,935 | 16,218 | 17,801 | 19,989 |
| bounds (ADR-385) | 0 | 0 | 0 | 0 |
| proofs (ADR-269 D4, ADR-396 D5) | 332,098 | 248,021 | 238,360 | 273,062 |
| views (NK2302, NK2305, NK2303) | 2,449,744 | 1,114,172 | 625,034 | 249,300 |
| lints (NK1261, NK1350) | 2,568 | 1,440 | 693 | 553 |
| rest of the walk | 2,196,567 | 1,209,843 | 734,726 | 398,662 |
| the walk | 4,999,242 | 2,589,904 | 1,616,711 | 941,749 |

A cost fixed per file makes that ratio fall as files grow, even where the
cost grows with the lines. So each analysis is fitted as `fixed + per line
× lines`, by least squares over all 274 files. The slope is also fitted
alone over the smaller and the larger half of the files by lines:

| Analysis | Fixed per file | Per line | Per line, smaller half | Per line, larger half |
| :--- | ---: | ---: | ---: | ---: |
| handles (ADR-312) | -11,944 | 189 | 52 | 202 |
| outside data (ADR-396) | 116,778 | 18,861 | 15,468 | 18,587 |
| bounds (ADR-385) | 0 | 0 | 0 | 0 |
| proofs (ADR-269 D4, ADR-396 D5) | -4,247,843 | 271,672 | 217,940 | 274,214 |
| views (NK2302, NK2305, NK2303) | 358,543,347 | 60,850 | 38,750 | 61,354 |
| lints (NK1261, NK1350) | 211,314 | 503 | 514 | 551 |
| rest of the walk | 320,894,061 | 225,442 | 406,827 | 222,174 |
| the walk | 675,505,713 | 577,518 | 679,552 | 577,082 |

Slopes over narrower ranges of the larger files, from the same 274 rows:

| Analysis | 440-1,499 lines (104 files) | 1,500-3,999 (29) | from 1,500 (33) | from 3,000 (8) |
| :--- | ---: | ---: | ---: | ---: |
| outside data | 20,461 | 17,554 | 17,166 | 15,703 |
| proofs | 326,062 | 198,013 | 273,188 | 290,662 |
| views | 64,851 | 51,731 | 56,934 | 51,645 |
| lints | 325 | 516 | 564 | 518 |
| rest of the walk | 258,375 | 227,384 | 215,503 | 192,322 |
| the walk | 670,196 | 495,213 | 563,533 | 550,822 |

### Verdict per analysis

In the file's lines, every analysis is **linear**. From about 440 lines up,
no slope rises with file size. The smaller half's lower slopes come from
the few hundred lines it spans, where the content of each file weighs more
than its size.

* **handles (ADR-312)**: linear. It costs about 190 instructions per line,
  and its cost follows the content: `prover_walk.nika` alone has 12.8 M of
  the 15 M in the sample.
* **outside data (ADR-396)**: linear, about 19,000 per line, with no
  fixed cost.
* **bounds (ADR-385)**: not measured. The unsigned-slot pass does not run
  in `lower-std --findings`, so every row is 0.
* **proofs (ADR-269 D4, ADR-396 D5)**: linear, about 272,000 per line,
  with no fixed cost. `check_facts.nika` is the outlier, at 915,000 per
  line.
* **views (NK2302, NK2305, NK2303)**: linear, about 61,000 per line, plus
  **359 M fixed per file**.
* **lints (NK1261, NK1350)**: linear, about 500 per line.
* **rest of the walk**: linear, about 225,000 per line, plus **321 M fixed
  per file**.

**The fixed cost per file is where the package grows faster than its
lines.** It is 676 M per file, about 185 G (61 %) of the 305 G that the
file checks cost. The 35-line `sources.nika` costs 742 M. The callgrind
inclusive costs of its dump (`files.callgrind.275`) show where:

* `views::result_package` (`views.nika`) costs 215 M. It goes over every
  function of the package's ledger (`asked.own.functions`).
* `check_core_place_returned` (`check_core_stmt.nika`) costs 210 M,
  through `check_core_returned_beside` and `names_in`. It walks the files
  beside.

Both are worked out again in each file's check. So the file checks cost
about files × package on top of the linear part, and grow with the square
of the package as it gets more files.
