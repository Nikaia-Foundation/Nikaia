1. **Most time:** the brief's test run, about 6 min of `it` plus the 7 `comptime_*` programs. The one rework was clippy's `while_let_loop` on the lowered `run_fingerprint` loop, which cost a second clippy run. The run took about 25 turns in all.
2. **Looked up, could have been handed:**
   - Run 2's commit adds no row to `scripts/self_hosting.py`. I had to grep for that, then find the right spot.
   - The format of the plan row's "built" cell. I copied K3a's.
   - The tools module's allow list in `lib.rs`.
   - Whether 1a5fdf6d is on HEAD. `--grep='#1106'` found the architect's batch commit, not the fix, so I checked the hash.
3. **Checks run:**
   - The two `lower-std` runs and the build, plus a third pass for the fixpoint (it changed nothing).
   - The `it` filters and `comptime_*` in a scratch project.
   - `--lib comptime_run::`: wasted, the module has 0 tests.
   - The `run_*` tests: there are none in `tests/compiler`.
   - clippy on 1.97, then on 1.99.1 in a separate target dir, about 10 min. That one only confirmed green, but the rules require it.
   - fmt and pre-push.
4. **One change:** a run-N brief that cherry-picks a wip commit should say which bookkeeping that commit lacks (the `self_hosting.py` row, the plan row, a lint the lowered file is known to trip). It should also drop test names that match nothing (`run_*`, `--lib comptime_run::`).
