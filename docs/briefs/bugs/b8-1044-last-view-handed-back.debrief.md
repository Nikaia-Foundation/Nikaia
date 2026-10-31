1. **Most turns and time** went to the whole `tests/compiler` package, which the gate runs in full whenever a `check_*.nika` changes. I ran it three times:
   - The three runs failed on 1, 11 and 1 tests, each time different ones.
   - The third run broke off when another agent rebuilt `target/fast/nikaia` under it.
   - All 14 tests that failed pass together at both settings.

   Before that, the costliest was the fallout of putting element reads in `view_of`: `a_changing_view` also reads `view_of`, so `lower-std` refused `emit_sums.nika` with NK2108. Fixing it took a bootstrap round (`lower-std` with the old binary, then a rebuild). Row e cost a probe cycle too: the brief's `read_a_map` is set only by `get`, not by `m[k]`.
2. **Looked up, which the brief could have handed me:**
   - `view_of` has a second reader, `a_changing_view` in `check_takes.nika`.
   - `facts.method_callees` keys (`Vec::last`, `HashMap::get`, ...) and `facts.map_read_views` for map brackets.
   - `lowered.rs` already held another agent's hunk, so my new helper had to be a method, which stays out of the index.
   - `commit.nika` now leaves the commit local (`Pushed: local`), while builder.md and the brief still say it pushes.
3. **Checks run:**
   - Probe programs a–i, each on the old and the new binary.
   - `fmt --check` on both new tests, and the `lower-std` fixpoint.
   - 14 `it` filters (309 passed) and `nikaia-std --lib check_`.
   - The gate (build, lower-std, fmt, clippy and pre-push green), then the guards on their own (green, sequences included).
   - A one-test package copy for each failing test.

   Wasted: the second and third whole-package runs. Shared-machine noise was already the likely cause; running only the failing tests settled it.
4. **One change:** builder.md should say that, while the driver runs the full suite and pushes (the current `commit.nika`), the gate's `test` step runs only the touched tests even when `check/` changed. A whole-package red that a one-test copy can't reproduce should be reported as flaky, not as `Needs: fix`. The brief should also list every reader of a field it tells you to extend (here `view_of` → `a_changing_view`).
