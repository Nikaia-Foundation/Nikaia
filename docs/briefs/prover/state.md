# Prover lane: state

*Current state only, rewritten by the lane's architect each run. What happened is in `git log` and the issues.*

**Landed (owner, 2026-10-31).** No batch queued (`batch.txt` empty). Epic #1053 (`ready`, `wip` removed; its last comment holds the status of each *To build* step).

**Goal (owner, 2026-10-31):** 100 % of the indexes and arithmetic steps in `crates/nikaia-std/src/tools/*.nika` proved, and `left at run time` 0: claim (b) of `docs/formal-verification.md`. Then the gaps outsiders name (lengths, sorted lists, facts about data, postconditions, lemmas). Then, by the score: the prover as a product, interfaces for outside proposers (ADR-361 D6, `nikaia prove --propose`), current research.

## How a new session restarts the lane

1. `git fetch origin develop && git checkout -B develop origin/develop` (CLAUDE.md); check p12 (ae09d77, "every length is an i64") is on `origin/develop`: at landing its suite was running and it was local only (`unpushed.txt`). If not pushed, the driver's next suite run pushes it.
2. `cargo build --profile fast -p nikaia`.
3. Queue the next brief: write `p13-1053-sequence-counts.md` into `batch.txt` (re-run its probe first: written on the 21:11 binary), or let the architect plan: first remove the *Landing* section from `architect-task.md`, or its run plans nothing.
4. `target/fast/nikaia run scripts/drive.nika -- docs/briefs/prover --max 8 --ignore-ci-red` with `run_in_background`.

## Figure (after p12, ae09d77: `lower-std --bounds --asserts` + `scripts/proved.nika`)

| Setting | Indexes | Operations | Left at run time |
| :--- | :--- | :--- | :--- |
| `off` | 1,544 / 2,677 (57.7 %) | 923 / 1,633 (56.5 %) | 1,846 |
| `prove-through-loops:on` | 1,576 / 2,677 (58.9 %) | 929 / 1,633 (56.9 %) | 1,808 |

Lane start (p1): 1,929 / 1,891 left. Claims: 0 asserts in the tools. Panics: the figure says 3, really 2 (#1103 step 5). Unproved by reason (on, before p12; p12 took counters 439 -> 359): 708 catch-all, 383 parameter, 128 list element, 112 another list's length, 61 call result, 37 list of lists, 7 `if`/`match` value, 6 product, 4 lower bound, 3 field.

## Lane facts (for the architect and briefs)

* Count: `target/fast/nikaia lower-std --bounds --asserts [--optimization prove-through-loops:on] > R` (about 45 s), then `target/fast/nikaia run scripts/proved.nika -- R`. Never per-file `check --bounds` on tools files. In `R` a file's header is `bounds in <path>: ...` in column 0; a site line is `   LINE  EXPR   proved|checked: REASON`.
* A program test carries `NAME.bounds` / `NAME.loops.bounds` (p5, #1069); every prover brief guards its gain with one. A claim test uses `.stderr` (NK1300/NK1207).
* Bounds walk: `bounds_walk.nika` (`bounding_length_named` :1728 the length axioms, `bounding_built` :3589 a `let`'s length, `bounding_each` :1639 the ledger's per-item facts), `bounds_matched.nika` `bm_each`. Prover walk (claims): `prover_walk.nika`, `prover_state.nika` (`length_given` :968), `prover_loops.nika`. Checker facts: `check_facts.nika`, `std_lengths`/`sized_lengths` set in `check_core_method.nika` :917 and `check_core_stmt_for`, `takes_space` in `check_decls.nika`.
* Certificate checker: `logic_search.nika` `lg_verify` and callees, `logic_lia.nika` (54 unproved), `logic_search.nika` (18), `logic_alethe.nika` (21). Its sites are mostly list elements used as indexes (`held[place]`, `needed[from]`: certificate data, so a guard that rejects a malformed certificate is right there), #388 pairs, parameters.
* Where the bounds walk stops (2^62 goal cap, no local types, loop guesses from halves of entry facts, `if` join): `docs/formal-verification.md` §2; builder traps of this lane: `docs/builder-facts.md` *Traps already met* (*Prover (bounds) work*).
* Valid `Facts:` headings: `Where things live`, `Traps already met`, `Commands`, `Moving Rust into Nikaia`.

## Order of work (score = (S x 10 M + O) / C)

| # | Work | Issue | S | O | C | Score |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 1-12 | Count, in-loop asserts, reasons, D10, `.bounds` tests, D13, `for` length, p8, p10, p11, p9 (claims, panics in the figure), p12 (ADR-385 D1, cross-file structs take space) | #1053.1-4/9, #1054, #1069, #1076, #1084, #1090 | | | | done |
| 13 | p13 (brief written): ADR-385 D2 `ensures_count` on split/chars/bytes, the panic count | #1103 | 3 | 3 | 4 | 8.3, first |
| 14 | #388 fill case: lists pushed once each per turn, or built from another's length, are as long as each other; decide (ADR), then brief | #388 | 3 | 3 | 5 | 6.6 |
| 15 | Catch-all re-split by form, an example per reason in `proved.nika` (saves the hand sort every architect run did) | #1105 | 2 | 2 | 4 | 5.5 |
| 16 | Certificate checker and the 2 panics at zero | #1104 | 3 | 1 | 6 | 5.2 |
| 17 | Relational counter candidate for a name grown on some paths (ADR-361 D5 item 2); the bounds walk guessing from the prover's closed forms (p2's finding) | #1053.4 | 3 | 2 | 6 | 5.3 |
| 18 | `assert`s at the 383 parameter sites, file by file, clear of the other lanes | #1053.8 | 3 | 1 | 10 | 3.1 |
| 19 | Postconditions on a program's own function (61 call-result sites): decision paper first | #1053.6 | | | | by the table |
| 20 | By the table: #797, #798 (decision), #384, #385, #386 | | | | | by the table |

## Decisions in force

* ADR-306 D12: a length is below 2^60 where elements take space. ADR-385 (architect, 2026-10-31, obvious): D1 every length <= 2^63 - 1; D2 `ensures_count` on a `std` sequence (p12, p13 build it; Part III text with each build).
* ADR-361 D2, D3, D10, D13 (a `for` counts its turns; in the bounds walk p6, the collection length via the checker p7; the prover walk's `for` builds a record only over a range).
* #1076: a call through `CallReach::Through` counts as not proving a precondition. #1090 (D15): a carried precondition is proved only if no call along its carrying chain leaves it open.
* #1084: `Bytes(x)` has `x`'s length; `[v; n]` has `n`; a literal argument its count. Tests are rebuilt so the prover reads the length, never `unproved-claims = "warn"`.
* #1069: `NAME.bounds` (off) and `NAME.loops.bounds` (on) beside a program test.
* p4: `bounding_fits` may state a one-way goal as `t <= y`, at most 8 per goal.
* Open decisions this lane may need: #388 fill case (label `decision`; the architect may find it obvious, see the comment there), #385, #386, #798; postconditions on functions (paper when item 19 comes).
* NK codes: `NK1320`-`NK1329`, `NK2720`-`NK2729` (none used yet).

## CI and the tree (landing, 2026-10-31)

p1-p11 and p9 pushed; p12 (ae09d77) committed locally, its suite running at landing. `lowered/project_members.rs` is residue (#1060): never stage it. Defects this lane found went to the bug lane: #1074 (`opt ?? other` over `ref T`, p6), #1081 second shape (view of a temporary's field, p9). Owed to the supervisor (version heading): the `Changelog:` lines of p5-p12; ADR-385 lands with p12's (D1) and #1103's (D2) Part III text.
