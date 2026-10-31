# Prover lane: state

*Current state only, rewritten by the lane's architect each run. What happened is in `git log` and the issues.*

**Goal (owner, 2026-10-31):** 100 % of the indexes and arithmetic steps in `crates/nikaia-std/src/tools/*.nika` proved: claim (b) of `docs/formal-verification.md`. Epic: #1053 (`wip`). Then the gaps outsiders name (lengths, sorted lists, facts about data, postconditions, lemmas). Then, by the score: the prover as a product, interfaces for outside proposers (ADR-361 D6, `nikaia prove --propose`), current research.

**The headline counts at `prove-through-loops:on`** (architect, 2026-10-31): claim (b) is about what is proved, the strongest setting the toolchain offers; which proved checks a build drops is a separate setting (ADR-361 D11: `std` and the tools are lowered with it `off`). Both figures are recorded. **The figure must also count claims** before `assert`s go into the tools: a precondition (ADR-269 D5) moves a check to its callers, and an unproved one there is still a run-time check.

## Figure (2026-10-31, 58a1ff9, p7's report: `lower-std --bounds` + `scripts/proved.nika`)

| Setting | Indexes | Operations |
| :--- | :--- | :--- |
| `off` | 1,531 / 2,648 (57.8 %) | 835 / 1,622 (51.5 %) |
| `prove-through-loops:on` | 1,563 / 2,648 (59.0 %) | 841 / 1,622 (51.8 %) |

Counter reason 470 (off), 433 (on), was 541. The rest by reason as at ecf435a (693 catch-all, 379 parameter, 127 list element, 96 another list's length, 60 call result, 37 list of lists...). `tools/*.nika` holds no `assert` and 13 `panic(` in 9 files: p9 puts both into the figure (`left at run time`, the lane's target: 0).

## Lane facts (for the architect and briefs)

* Count: `target/fast/nikaia lower-std --bounds [--optimization prove-through-loops:on] > R` (about 45 s), then `target/fast/nikaia run scripts/proved.nika -- R`. Never per-file `check --bounds` on tools files: alone they are refused. The report's file header lines start in column 0; `awk '/^[^ ]/{f=$0} /REASON/{print f": "$1}' R` maps sites to files.
* A program test carries `NAME.bounds` / `NAME.loops.bounds` (p5, #1069); every prover brief guards its gain with one. `--asserts` has no such guard: a claim test uses `.stderr` (NK1300/NK1207).
* The bounds walk (`bounds_walk.nika`) is separate from the prover walk (`prover_walk.nika`, `prover_state.nika`, `prover_loops.nika`). p4 gave the walk the portfolio for `while` only; p6 gave it D13 (`bounding_turn_steps`, `bounding_turn_bounds`), reading a collection's length from the checker's `std_lengths`/`sized_lengths` by `node_key`. The prover walk (`prover_for`, `prover_walk.nika` :1068) builds a record only for a `for` over a range.
* Certificate checker: `logic_search.nika` `lg_verify` and its callees, `logic_lia.nika`.
* Valid `Facts:` headings: `Where things live`, `Traps already met`, `Commands`, `Moving Rust into Nikaia`.
* `nikaia.toml` has no optimization key: a setting is chosen on the command line only.

## Order of work (score = (S x 10 M + O) / C)

| # | Work | Issue | S | O | C | Score |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 1-8 | Count, in-loop asserts, reasons, D10 portfolio, `.bounds` tests, D13 in the bounds walk, a `for`'s collection length (p7), `Through` calls not proving (p8) | #1053.1-4, #1054, #1069, #1076 | | | | done (local, unpushed) |
| 9 | p10: `Bytes(x)` keeps `x`'s length, a bound `[v; n]` has `n`, a literal or repeat argument its count; the four `it` tests p8 turned red rebuilt to use it | #1084 | 3 | 3 | 3 | 11, batch, first |
| 10 | p9: `lower-std --asserts`, `proved.nika` counts claims and `panic`s, `left at run time`; `formal-verification.md` §2/§4 | #1053.9 | 3 | 1 | 4 | 7.8, batch, after p10 |
| 11 | The prover walk's `for`: a turn count (D13) as `loop_record_range` over `0..len`, so claims after/inside a `for` read its counters, and a `push` once a turn in a `for` over `0..<n` gives the length `old + n` (probed: `for j in 0..<4 { u.push(j) }` then `g(u)` with `u.len() >= 4` refused at both settings) | #1053.4 | 3 | 3 | 8 | 4.1, next run |
| 12 | Certificate checker at zero: preconditions as `assert`s, guards for malformed certificates | #1053.5 | 3 | 1 | 6 | 5.2, after 10-11 |
| 13 | The catch-all's 693 re-split (p7 landed) | #1053.3 | 2 | 2 | 4 | 5.5, with p9's run |
| 14 | `assert`s at the 379 parameter sites, file by file, files clear of the main lane | #1053.8 | 3 | 1 | 10 | 3.1, after 10-11 |
| 15 | The 13 `panic`s: each a proved-unreachable claim or a `throw` | #1053 | 2 | 1 | 4 | 5.3, after 10 |
| 16 | By the table: list elements #797, other list's length #388 (decision), call results (postconditions: decision paper due), #384, #385, #386 | | | | | by the table |

## Decisions in force

* ADR-306 D12: a length is below 2^60, so `xs.len() + 1` is proved.
* ADR-361 D2, D3, D10: claims and checks are asked through loops (p2, p4). D13 (architect, 2026-10-31): a `for` counts its turns; a counter is bounded by `k` times them, at both settings; built in the bounds walk (p6), the collection length via the checker (p7), the prover walk next (item 10).
* p7 (architect, 2026-10-31): `check_core_stmt_for` records the iterable in `std_lengths` (and `sized_lengths` where elements take space), as `.len()` does. `for c in text` over a `String` passes check and fails in rustc: #1075 (bug lane).
* #1076 (architect, 2026-10-31): a call that reaches the checked entry (`CallReach::Through`, ADR-269 D20) checks the precondition when it runs; it counts as not proving it.
* #1084 (architect, 2026-10-31, obvious): `Bytes(x)` has `x`'s length (like `find_all`, ADR-375 D5); `[v; n]` has length `n` bound by `let` or passed; a literal argument has its count. Tests p8 turned red are rebuilt so the prover reads the length, never lowered with `unproved-claims = "warn"` to pass.
* #1069 (architect, 2026-10-31): `NAME.bounds` (off) and `NAME.loops.bounds` (on) beside a program test, path relative, the build-dependent tail cut.
* p4: `bounding_fits` may state a one-way goal as `t <= y` for a name `y` whose type lies inside the operation's, at most 8 per goal.
* Open decisions this lane may need: #388 (equal lengths), #385 (lists of lists, `decision`), #386 (products), #798 (element rules), postconditions (`ensures`, no issue yet: write the decision paper when item 12 reaches call results).
* NK codes: `NK1320`-`NK1329`, `NK2720`-`NK2729`.

## CI and the tree (2026-10-31, after 3feaff4)

Local `develop` holds p7 (58a1ff9) and p8 (788981d) unpushed: the driver's suite is red on four `it` tests p8 rightly refuses (NK1300, true unproved calls through `Bytes(v)`); p10 makes them green by #1084, then the driver pushes. `RED`/`qa-input.txt` in this directory are an older QA round's (rs2nika/examples), not this lane's. `lowered/project_members.rs` is uncommitted residue (#1060, b15); briefs say never to stage it. Owed to the supervisor (version heading): p5's, p6's, p7's and p8's `Changelog:` lines.

## Batch

`batch.txt`: p10 (`prover_walk.nika`, `prover_state.nika`, two `it` test files, new `tests/compiler` tests), then p9 (`main.rs`, `sysroot.rs`, `scripts/proved.nika`, `formal-verification.md`). None touches a file of the main, bug or yaml lane's current briefs. Next run: item 11 (the prover walk's `for`, push lengths), then 12 or 14 by p9's figure.
