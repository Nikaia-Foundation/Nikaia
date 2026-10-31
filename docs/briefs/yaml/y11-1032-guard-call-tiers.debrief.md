1. **Most time:** the `nikaia test --both-settings --project tests/compiler` run, which took over 10 minutes in the background. The second most was the gate plus clippy, about a minute of the gate's tests plus clippy's build beside it. The fix itself was 5 lines and took one build cycle. Turns went only to reading, the one cycle and the long runs.
2. **Looked up:** I read the whole of `docs/builder-facts.md` (the one read the brief asks for). I also looked up the `Pattern::Guard` field names in `ast.nika` (`cond`, `expected`) and how `tier_expr` is called on a match guard. The brief gave the line number for the call but not the variant's shape. One line in the brief, `Pattern::Guard { cond: Expr, expected: String? }`, would have saved that grep. No ADR or converter was needed.
3. **Checks run:**
   - The probe on the old binary (needed: it showed the test failing).
   - One build cycle with two `lower-std` runs (needed).
   - The probe on the new binary.
   - The six-guard gate with clippy beside it.
   - `pre-push-check`.
   - The full `tests/compiler` run at both settings.
   - A separate `sequences::` run, to see the guard's figure. It was wasted: the gate run had already reported it `ok`.

   The full `tests/compiler` run is required for `check/` changes but arguably oversized for a `text_tiers.nika` fix. Running only the `grammar_*` tests plus the new one would have given the same signal much sooner. `nikaia test` has no name filter, though.
4. **One change:** let `scripts/commit.nika` handle a rejected push. Either it does a `git pull --rebase` itself when the tree allows, or the brief states plainly that a rejected push counts as done. As it is, my run ends "PUSH REJECTED" even though the work is finished, and the report must carry `Pushed: rejected`. Second choice: state in builder.md that the `tests/compiler` run is needed only when a change can alter a lowering beyond the touched pass. A fix in one tier pass could then run just the tests that match its name.
