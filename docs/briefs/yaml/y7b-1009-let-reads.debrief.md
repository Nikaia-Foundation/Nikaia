1. **Most time:** the `tests/compiler --both-settings` run, which was the one long wait. After that came two extra cycles I caused myself:
   - My new `return` path moved `called_summed` early (NK2105), which cost one lower-std/build cycle.
   - Clippy's `collapsible_if` fired on the lowered code of an `if` I had nested. That cost a second clippy run just to find the line, because my `grep '^(error|warning)'` filter hid the `-->` location. Then one more cycle.

2. **Looked up that the brief could have handed me:**
   - That `walk.state.readers` (`prover_readers`) already exists and is the right model for defect 2.
   - That `std` contracts record `mut` parameters in `Signature.mutable`, so `scalar::from` could count as a reader.
   - What `scope.rebind` clears, and that the `Stmt::Assign` handling rebinds names that are not in `mutables`.
   - That `prover_follow_mutables`, not `prover_let`, decides whether a `let mut` is followed. Defect 3 needed both functions, and the brief named only `prover_let`.
   - That a `match` is never joined. I found that from an edge probe.

3. **Checks run:**
   - **Useful:** six probes on the old binary before editing; edge probes; the `it` filters plus the guard set (203); clippy; `tests/compiler` (384); `packages/yaml` (9); pre-push.
   - **Wasted:** a second pre-push run only to read its exit code; the extra clippy run caused by my filter; a "fixpoint" rebuild that did nothing.
   - **Gap:** I never ran the six committed test files on the unfixed compiler. I only ran the same probe bodies, and I had overwritten the old binary before I thought to keep a copy.

4. **One change:** builder.md should say "`cp target/fast/nikaia $S/nikaia.before` before your first build". That makes the required "fails on the unfixed compiler" check one command, with no stash and no rebuild. It should also say "clippy lints in lowered code: grep with `-A3` for `-->`", or better, tell builders to flatten nested `if` in `tools/*.nika` up front.
