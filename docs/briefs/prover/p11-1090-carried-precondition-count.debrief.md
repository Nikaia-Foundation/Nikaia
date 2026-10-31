1. **Most cost:** a second build-and-test round. The `it` test `a_broken_precondition_names_the_caller` broke because `check_claims_unproved` skips calls NK1207 already refuses, but only for calls to the `assert`'s own function. The brief did not mention that rule. Smaller losses: a scratch project without `src/main.nika` crashed `nikaia test` (one wasted run), and `nikaia check` on a file inside `tests/compiler` checked the whole project, so it showed another test's parse error.
2. **Looked up:**
   - The NK1207 skip in `check_claims.nika`, and that refusal findings are matched by message text.
   - That `carried_site_of` is private.
   - That a map read `m[k] ?? fallback` hands back a view, so the fallback must be a view too (NK1185).
   - That `nikaia test` needs a `src/main.nika`.

   The brief had the cause and file locations right; that saved most of the reading.
3. **Checks:**
   - The probes on the old and new binary, the 27 `assert(` programs with `--both-settings` (run twice, once per build), the `it` filters, the `lower-std` fixpoint, clippy on 1.97 and 1.99.1, fmt and `pre-push-check.sh`.
   - Wasted: the first scratch test run, which crashed for lack of `src/`. Running the 27 programs before the `it` filters (seconds) meant re-running them after the second fix. Clippy on 1.99.1 for a change that only touched a lowered file was probably not needed.
4. **One change:** in the brief, name `check_claims.nika` `check_claims_unproved` (its NK1207 skip) as a related reader, and say: "run the `it` filters before `tests/compiler`; they take seconds."

   In builder-facts: the one-program scratch recipe should also create `src/main.nika` (`fn main() {}`). Also note that `nikaia check` on a file inside a project checks the whole project, so copy probes out first.
