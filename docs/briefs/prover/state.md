# Prover lane: state

*Current state only, rewritten by the lane's architect each run. What happened is in `git log` and the issues.*

**Goal (owner, 2026-10-31):** 100 % of the indexes and arithmetic steps in `crates/nikaia-std/src/tools/*.nika` proved: claim (b) of `docs/formal-verification.md`. Epic: #1053 (`wip`). Then the gaps outsiders name (lengths, sorted lists, facts about data, postconditions, lemmas). Then, by the score: the prover as a product, interfaces for outside proposers (ADR-361 D6, `nikaia prove --propose`), current research.

**The headline counts at `prove-through-loops:on`** (architect, 2026-10-31): claim (b) is about what is proved, the strongest setting the toolchain offers; which proved checks a build drops is a separate setting (ADR-361 D11: `std` and the tools are lowered with it `off`). Both figures are recorded. **The figure must also count claims** before `assert`s go into the tools: a precondition (ADR-269 D5) moves a check to its callers, and an unproved one there is still a run-time check.

## Figure (2026-10-31, 76c216e binary: `lower-std --bounds` + `scripts/proved.nika`)

| Setting | Indexes | Operations |
| :--- | :--- | :--- |
| `off` | 1,536 / 2,659 (57.8 %) | 838 / 1,625 (51.6 %) |
| `prove-through-loops:on` | 1,568 / 2,659 (59.0 %) | 844 / 1,625 (51.9 %) |

Unproved by reason (on): 702 catch-all, 433 counter, 382 parameter, 127 list element, 110 another list's length, 61 call result, 37 list of lists, 7 `if`/`match` value, 6 product, 4 lower bound, 3 field. Top files: `fmt.nika` 135, `text_tiers.nika` 87, `bounds_walk.nika` 65, `logic_lia.nika` 54. `tools/*.nika` holds no `assert` and 13 `panic(` in 9 files: p9 puts both into the figure (`left at run time`, the lane's target: 0).

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
| 1-9 | Count, in-loop asserts, reasons, D10 portfolio, `.bounds` tests, D13 in the bounds walk, a `for`'s collection length, `Through` calls not proving (p8), argument lengths (p10) | #1053.1-4, #1054, #1069, #1076, #1084 | | | | done (p10 local, unpushed) |
| 10 | p11: a precondition carried back (D15) counts the unproved calls of every function it was carried to | #1090 | 3 | 3 | 3 | 11, batch, first |
| 11 | p9: `lower-std --asserts`, `proved.nika` counts claims and `panic`s, `left at run time`; `formal-verification.md` §2/§4 | #1053.9 | 3 | 1 | 4 | 7.8, batch, after p11 |
| 12 | The prover walk's `for`: a turn count (D13) as `loop_record_range` over `0..len`, so claims after/inside a `for` read its counters, and a `push` once a turn in a `for` over `0..<n` gives the length `old + n` (probed: `for j in 0..<4 { u.push(j) }` then `g(u)` with `u.len() >= 4` refused at both settings) | #1053.4 | 3 | 3 | 8 | 4.1, next run |
| 13 | Certificate checker at zero: preconditions as `assert`s, guards for malformed certificates (`logic_lia.nika` 54 unproved) | #1053.5 | 3 | 1 | 6 | 5.2, after 11-12 |
| 14 | The catch-all's 702 re-split | #1053.3 | 2 | 2 | 4 | 5.5, with p9's run |
| 15 | `assert`s at the 382 parameter sites, file by file, files clear of the main lane | #1053.8 | 3 | 1 | 10 | 3.1, after 11-12 |
| 16 | The 13 `panic`s: each a proved-unreachable claim or a `throw` | #1053 | 2 | 1 | 4 | 5.3, after 11 |
| 17 | By the table: list elements #797, other list's length #388 (decision), call results (postconditions: decision paper due), #384, #385, #386 | | | | | by the table |

## Decisions in force

* ADR-306 D12: a length is below 2^60, so `xs.len() + 1` is proved.
* ADR-361 D2, D3, D10: claims and checks are asked through loops (p2, p4). D13 (architect, 2026-10-31): a `for` counts its turns; a counter is bounded by `k` times them, at both settings; built in the bounds walk (p6), the collection length via the checker (p7), the prover walk next (item 10).
* p7 (architect, 2026-10-31): `check_core_stmt_for` records the iterable in `std_lengths` (and `sized_lengths` where elements take space), as `.len()` does. `for c in text` over a `String` passes check and fails in rustc: #1075 (bug lane).
* #1076 (architect, 2026-10-31): a call that reaches the checked entry (`CallReach::Through`, ADR-269 D20) checks the precondition when it runs; it counts as not proving it.
* #1084 (architect, 2026-10-31, obvious): `Bytes(x)` has `x`'s length (like `find_all`, ADR-375 D5); `[v; n]` has length `n` bound by `let` or passed; a literal argument has its count. Tests p8 turned red are rebuilt so the prover reads the length, never lowered with `unproved-claims = "warn"` to pass.
* #1090 (architect, 2026-10-31, defect): a claim held as a precondition is proved only if no call proves it short anywhere along its carrying chain (D15): its unproved calls are its own function's plus those of every function its precondition was carried to.
* #1069 (architect, 2026-10-31): `NAME.bounds` (off) and `NAME.loops.bounds` (on) beside a program test, path relative, the build-dependent tail cut.
* p4: `bounding_fits` may state a one-way goal as `t <= y` for a name `y` whose type lies inside the operation's, at most 8 per goal.
* Open decisions this lane may need: #388 (equal lengths), #385 (lists of lists, `decision`), #386 (products), #798 (element rules), postconditions (`ensures`, no issue yet: write the decision paper when item 12 reaches call results).
* NK codes: `NK1320`-`NK1329`, `NK2720`-`NK2729`.

## CI and the tree (2026-10-31, after 28ffe0b)

p7 and p8 are pushed. Local `develop` holds p10 (76c216e) unpushed; the driver cut the batch on p10's `Found:` (#1090, filed and briefed as p11), so p9 did not run; the batch's suite was starting on 76c216e at 20:34. `RED`/`qa-input.txt` in this directory are an older QA round's, not this lane's. `lowered/project_members.rs` is residue (#1060); briefs say never to stage it. `sysroot.rs` is shared with the bug lane's b17 (#1077): p9 stages its own hunks only. Owed to the supervisor (version heading): p5's-p8's and p10's `Changelog:` lines.

## Batch

`batch.txt`: p11 (`prover_passes.nika`, maybe `prover_claims.nika`, two new `tests/compiler` tests), then p9 (`main.rs`, `sysroot.rs`, `scripts/proved.nika`, `formal-verification.md`). Next run: item 12 (the prover walk's `for`, push lengths), then 13 or 15 by p9's figure.
