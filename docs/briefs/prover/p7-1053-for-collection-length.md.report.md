Result: done
Files: crates/nikaia-std/src/tools/check_core_method.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/check_core_method.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs tests/compiler/tests/prove_for_counts_its_turns.nika tests/compiler/tests/prove_for_counts_its_turns.stdout tests/compiler/tests/prove_for_counts_its_turns.bounds tests/compiler/tests/prove_for_counts_its_turns.loops.bounds tests/compiler/tests/prove_loop_counter_bounds.nika tests/compiler/tests/prove_loop_counter_bounds.bounds tests/compiler/tests/prove_loop_counter_bounds.loops.bounds
Commit: prover: a `for` over a collection records its length, so D13's turn count bounds its counters
Body: `check_core_stmt_for` records the iterable in `facts.std_lengths` (and `sized_lengths` where the elements take space) when `check_core_method_counted` holds (now `pub`), as `.len()` does; `bounds_walk.nika` picks the fact up unchanged (`bounding_length` at the `for` turn bound). prove_for_counts_its_turns: `every`, `some`, `same` (index and step) and the new `entries` (a `HashMap` walked as `(k, v)`) proved at both settings; the new `walked_chars` (`text.chars()`) and every soundness row (unbounded start, `u8 += 2`, reset, inner loop, doubling, `u8` over a range) stay checked. prove_loop_counter_bounds re-blessed: `c`'s `n += 1` rightly proved now, its header comment says so. Figure (lower-std --bounds, scripts/proved.nika): off 1,531/2,648 indexes, 835/1,622 operations; prove-through-loops:on 1,563/2,648, 841/1,622; the counter reason 470 off, 433 on (was 541 on).
Part: #1053
Tests: tests/compiler --both-settings 483 passed 0 failed; cargo test it -- prove bounds 179 passed; lower-std fixpoint clean; clippy 1.97 and 1.99.1 clean; fmt; pre-push-check; figure off 1531/2648 idx 835/1622 ops, on 1563/2648 idx 841/1622 ops, counter reason 433 on (was 541); unfixed binary probed: the four rows checked before
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: The prover counts a `for` over a list, map or set as one turn per element, so a counter stepped once a turn in such a loop is proved within the collection's length and loses its overflow check.
Owner: none
Rs2nika: none
Disk freed: 0.65 GB (clippy 1.99 target removed); cache 4.4G, 11G free, new entry run-bundle 480 MB
Pushed: local 58a1ff936cfc7f79278dc8329d666be831c93154
