# Prover lane: state

*Current state only, rewritten by the lane's architect each run. What happened is in `git log` and the issues.*

**Goal (owner, 2026-10-31):** 100 % of the indexes and arithmetic steps in `crates/nikaia-std/src/tools/*.nika` proved, and `left at run time` 0: claim (b) of `docs/formal-verification.md`. Epic: #1053 (`wip`). Then the gaps outsiders name (lengths, sorted lists, facts about data, postconditions, lemmas). Then, by the score: the prover as a product, interfaces for outside proposers (ADR-361 D6, `nikaia prove --propose`), current research.

**The headline counts at `prove-through-loops:on`**: claim (b) is about what is proved; which proved checks a build drops is a separate setting (ADR-361 D11: `std` and the tools are lowered with it `off`). Both figures are recorded.

## Figure (2026-10-31, 21:11 binary at 9e657f7: `lower-std --bounds --asserts` + `scripts/proved.nika`)

| Setting | Indexes | Operations | Left at run time |
| :--- | :--- | :--- | :--- |
| `off` (p9, a784e6d) | 1,543 / 2,676 (57.7 %) | 840 / 1,633 (51.4 %) | 1,929 |
| `prove-through-loops:on` | 1,576 / 2,677 (58.9 %) | 846 / 1,633 (51.8 %) | 1,891 |

Claims: 0 asserts in the tools. Panics: the figure says 3, really 2 (`modules.nika` :496 is a `panic(` in generated text; p13 step 5 fixes the count). Unproved by reason (on): 708 catch-all, 439 counter, 383 parameter, 128 list element, 112 another list's length, 61 call result, 37 list of lists, 7 `if`/`match` value, 6 product, 4 lower bound, 3 field.

What the architect's sample (2026-10-31) found in the big buckets:

* **Counters (439)**: 82 `x += 1` in a `for` over a collection, 52 in a `while`; the same shapes prove in a one-file program. Two causes probed: an unsized length has no upper bound (ADR-385 D1, p12), and a list of a struct declared in another tools file is taken as unsized (suspected `takes_space` not seeing it, p12 step 2).
* **Catch-all (708)**: by shape 267 `xs[i]`, 160 `xs[K]`, 91 `n + K`. Of the indexed lists, about 27 come from `split(..).collect()`, 26 from `chars().collect()` (ADR-385 D2, p13), 5 from `splitn`; `found`-style indexes set inside a guarded loop (`found = index` under `index < len`, then `if found >= 0 { xs[found] }`) and call results returning an index or -1 (`the_boundary`) need an invariant / postcondition.
* **Another list's length (112)**: lists filled pairwise (`for _ in handles { held.push(false) }`, then `held[index]` under `index < handles.len()`): #388's fill case. Its parameter case already works: `assert(a.len() == b.len())` at the top is a precondition the walk uses (probed).

## Lane facts (for the architect and briefs)

* Count: `target/fast/nikaia lower-std --bounds --asserts [--optimization prove-through-loops:on] > R` (about 45 s), then `target/fast/nikaia run scripts/proved.nika -- R`. Never per-file `check --bounds` on tools files. In `R` a file's header is `bounds in <path>: ...` in column 0; a site line is `   LINE  EXPR   proved|checked: REASON`.
* A program test carries `NAME.bounds` / `NAME.loops.bounds` (p5, #1069); every prover brief guards its gain with one. A claim test uses `.stderr` (NK1300/NK1207).
* Bounds walk: `bounds_walk.nika` (`bounding_length_named` :1728 the length axioms, `bounding_built` :3589 a `let`'s length, `bounding_each` :1639 the ledger's per-item facts), `bounds_matched.nika` `bm_each`. Prover walk (claims): `prover_walk.nika`, `prover_state.nika` (`length_given` :968), `prover_loops.nika`. Checker facts: `check_facts.nika`, `std_lengths`/`sized_lengths` set in `check_core_method.nika` :917 and `check_core_stmt_for`, `takes_space` in `check_decls.nika`.
* Certificate checker: `logic_search.nika` `lg_verify` and callees, `logic_lia.nika` (54 unproved), `logic_search.nika` (18), `logic_alethe.nika` (21). Its sites are mostly list elements used as indexes (`held[place]`, `needed[from]`: certificate data, so a guard that rejects a malformed certificate is right there), #388 pairs, parameters.
* Valid `Facts:` headings: `Where things live`, `Traps already met`, `Commands`, `Moving Rust into Nikaia`.

## Order of work (score = (S x 10 M + O) / C)

| # | Work | Issue | S | O | C | Score |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 1-11 | Count, in-loop asserts, reasons, D10, `.bounds` tests, D13, `for` length, p8, p10, p11 (carried preconditions), p9 (claims and panics in the figure) | #1053.1-4/9, #1054, #1069, #1076, #1084, #1090 | | | | done, pushed |
| 12 | p12: ADR-385 D1 (every length <= 2^63 - 1), the tools' cross-file structs taking space | #1053 | 3 | 3 | 3 | 11, batch, first |
| 13 | p13: ADR-385 D2 (`ensures_count`: split, chars, bytes), the panic count | #1053 | 3 | 3 | 4 | 8.3, batch, second |
| 14 | #388 fill case: two lists pushed once each per turn (or built from one another's length) are as long as each other; decide (obvious: no syntax, every user gains; the parameter case is an `assert`), ADR, brief | #388 | 3 | 3 | 5 | 6.6, next run |
| 15 | The catch-all re-split in `bounds_reasons.nika`: the reason names the form (`found`-style index, call result, list from a `std` sequence without an entry) | #1053.3 | 2 | 2 | 4 | 5.5, next run |
| 16 | Certificate checker at zero: guards rejecting malformed certificates, `assert` preconditions its callers prove, after 12-14 | #1053.5 | 3 | 1 | 6 | 5.2 |
| 17 | The 2 `panic`s: each a proved-unreachable claim or a `throw` (`check_walked.nika` :121, `emit_grammar.nika` :225) | #1053 | 2 | 1 | 3 | 7, with 16 |
| 18 | `assert`s at the 383 parameter sites, file by file, clear of the other lanes | #1053.8 | 3 | 1 | 10 | 3.1 |
| 19 | Postconditions (`ensures` on a program's own function, ADR-269 D17 has methods): call results returning an index or -1 (61 sites); decision paper if a syntax is needed | #382 (closed), new | | | | by the table |
| 20 | By the table: list elements #797, #798 (decision), #384, #385, #386 | | | | | by the table |

## Decisions in force

* ADR-306 D12: a length is below 2^60 where elements take space. ADR-385 (architect, 2026-10-31, obvious): D1 every length <= 2^63 - 1; D2 `ensures_count` on a `std` sequence (p12, p13 build it; Part III text with each build).
* ADR-361 D2, D3, D10, D13 (a `for` counts its turns; in the bounds walk p6, the collection length via the checker p7; the prover walk's `for` builds a record only over a range).
* #1076: a call through `CallReach::Through` counts as not proving a precondition. #1090 (D15): a carried precondition is proved only if no call along its carrying chain leaves it open.
* #1084: `Bytes(x)` has `x`'s length; `[v; n]` has `n`; a literal argument its count. Tests are rebuilt so the prover reads the length, never `unproved-claims = "warn"`.
* #1069: `NAME.bounds` (off) and `NAME.loops.bounds` (on) beside a program test.
* p4: `bounding_fits` may state a one-way goal as `t <= y`, at most 8 per goal.
* Open decisions this lane may need: #388 (to decide next run, see item 14), #385, #386, #798; postconditions on functions (paper when item 19 comes).
* NK codes: `NK1320`-`NK1329`, `NK2720`-`NK2729` (none used yet).

## CI and the tree (2026-10-31, after 1612975)

The suite on 9e657f7 was green; p11 (3ebe381) and p9 (9e657f7) are pushed. CI runs for 1612975 in progress, the runs before green. `RED` (empty) and `qa-input.txt` in this directory are an older round's, not this lane's. p9's `Found:` (a view of a temporary's field in a `let`, rustc "temporary value dropped while borrowed") is #1081's second shape: commented there, bug lane. `lowered/project_members.rs` is residue (#1060): briefs say never to stage it. `sysroot.rs` is shared with the main lane's gr2. Owed to the supervisor (version heading): p5's-p11's and p9's `Changelog:` lines; ADR-385 lands with p12/p13's Part III text.

## Batch

`batch.txt`: p12 (`bounds_walk.nika`, maybe `check_decls.nika`/`sysroot.rs`, `30-nikaia-tooling.md`, `formal-verification.md`, one test), then p13 (`ty.nika`, `ledger.nika`, `ledger_text.nika`, `bounds_matched.nika`, `bounds_walk.nika`, `std.contracts`, `scripts/proved.nika`, `30-nikaia-tooling.md`, `formal-verification.md`, one test). Next run: #388's decision and brief (item 14), the catch-all re-split (15), then the certificate checker.
