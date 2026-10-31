1. **What cost the most.** Most of the wall-clock time went to waiting on `tests/compiler --both-settings` and the clippy 1.99.1 cold build. Of the turns, about 8 went to reading the checker to find the existing `got_past_a_jump` set, its readers, and the place `lent_lets` is cleared. Smaller losses:
   - `catch e { .. }` is wrong syntax; the caught value is the implicit `error`.
   - `load(input; limits: ..)` failed with no default. "Defaults written out" turned out to mean putting the default into the signature as a literal.
   - My first test held an `owned(a: A)` row that failed for an unrelated reason.

2. **What the brief could have handed me.**
   - The pointer to `got_past_a_jump` and `check_core_stmt_got`, so `lent_list_reads` / `lent_lets` would have been ruled out from the start. The suspected spot (`check_core_flow_through_a_view`) was correct.
   - The `MatchPattern` / `Expr::Match` shapes in `ast.nika`.
   - That the `catch` block binds the implicit `error`.
   - How to call a package's `load` from a scratch `main`.

3. **Checks run, and which were wasted.** I ran:
   - the probe matrix on the old binary;
   - one `lower-std` / build / `lower-std` cycle, with no fixpoint rebuild needed;
   - the probes and the test on my copy of the binary;
   - the yaml check and run, on both binaries;
   - `it` view tests, fmt, clippy 1.97, `pre-push-check.sh`, clippy 1.99.1, and `tests/compiler`.

   None was really wasted. The clippy 1.99.1 run was the weakest: the only Rust that changed was lowered code, whose lints are already allowed, and it cost a 650 MB cold build. One more slip: a `grep` of the newest run cache showed a stale entry and told me nothing.

4. **One change that would have saved the most.** In `builder-facts.md`, add a line: "a change that touches only `tools/*.nika` and `lowered/*.rs` skips the newer-clippy run (lowered lints are allowed)". After that comes a short *Nikaia syntax crib* in the facts: `catch { error }`, binding patterns are `MatchPattern::Path([x])`, and an option after `;` needs a default literal in a scratch run.
