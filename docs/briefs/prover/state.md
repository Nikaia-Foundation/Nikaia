# Prover lane: state

*Current state only, rewritten by the lane's architect each run. What happened is in `git log` and the issues.*

**Goal (owner, 2026-10-31):** 100 % of the indexes and arithmetic steps in `crates/nikaia-std/src/tools/*.nika` proved. This is claim (b) of `docs/formal-verification.md`. Epic: #1053 (`wip`). Then come the gaps outsiders name (lengths, sorted lists, facts about data, postconditions, lemmas). After that, by the score: the prover as a product, interfaces for outside proposers (ADR-361 D6 already decides `nikaia prove --propose`), and current research.

## Figure (2026-10-31, d206915, `check --bounds` per file)

1,821 indexes, 965 proved (53 %). 1,381 operations, 560 proved (41 %). 246 files. **Not reliable**: 227 files are refused when checked on their own (the package's types are unknown), so their sites are undercounted. Brief p1 replaces this with the package count (`nikaia lower-std --bounds`, `scripts/proved.nika`). Reasons today: 1,580 catch-all, 57 call results, 27 lists of lists, 6 products, 3 fields. 281 of the unproved operations are `x += 1`.

## Order of work (score = (S x 10 M + O) / C)

| # | Work | Issue | S | O | C | Score |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 1 | Package count and `proved.nika` (p1) | #1053.1 | 2 | 2 | 6 | 3.7 |
| 2 | In-loop claims through their own loop (p2) | #1054 | 3 | 3 | 10 | 3.3 |
| 3 | Reasons by binding (p3) | #1053.3 | 2 | 2 | 6 | 3.7 |
| 4 | Counters: relational candidate `n - n@ <= k(i - i@)`, `for` turns bounded by length | #1053.4, #432 | 3 | 3 | 12 | 2.8 |
| 5 | Certificate checker (`logic_search.nika` `lg_verify`..`lg_replayed`, `logic_lia.nika`) at zero: preconditions (`assert(at == made.len())` is proved at its call today), guards that reject malformed certificates | #1053.5 | 3 | 1 | 6 | 5.2, after 2 and 4 |
| 6 | Field facts #384 (ADR-306 D17), and by the p3 table: #388, #385, #386, postconditions | | | | | by p3's table |
| 7 | Facts about data: #797 (ready), #798 (decision) | | | | | |

Items 1 and 3 come before 2 and 4 because they choose the rest. Item 5 waits for 2 and 4, because most of its sites are counters and indexes inside loops.

## Decisions in force

* ADR-306 D12: a length is below 2^60, so `xs.len() + 1` is proved. #383 is built.
* ADR-361 D2, D3, D10: claims are asked through loops, inside and after them. Inside is not built (#1054).
* ADR-361 D5: the strategy portfolio. Closed forms, intervals and turn-indexed intervals are built. Octagon-like pair bounds are not.
* `tests/compiler` is one project, so per-test optimizations cannot be set. `prove-through-loops:on` results are shown with `check --bounds` under `Tests:`, not as `.nika` tests.
* Open decisions this lane may need: #386 (products), #388 (equal lengths), #385 (lists of lists, labelled `decision` 2026-10-31), #798 (element rules), and postconditions (`ensures`, no issue yet).
* NK codes: `NK1320`-`NK1329`, `NK2720`-`NK2729`.

## Batch

p1 → p2 → p3 (`batch.txt`). The files they touch (`main.rs` lower-std, `sysroot.rs`, `bounds_report.rs`, `bounds_reasons.nika`, `prover_state.nika`, `prover_loops.nika`, `bounds_walk.nika`) are not in the main lane's current batch (BT, s922, GR).
