1. **Most time:** waiting about 10 minutes for `nikaia test --both-settings --project tests/compiler`. The turns themselves were cheap, about 20 in all. One clippy round needed a second call because `$S` was unset in the subshell after `&`, so I had to `cat` the output file again.
2. **Looked up:** the `Ty` variants (`Tuple(Vec[Ty])`) and the body of `is_a_view` in `ty.nika`. One `grep` and one `sed` covered both. The brief's line numbers were exact and its trace was right, so I didn't need to look anything else up. It could have said directly that the contract writes the result as `(ref(s) String, i64)`, but checking that took a single `lower`.
3. **Checks run:**
   - Probes on the old binary: the issue program failed in rustc, and the optional form was refused by NK1104.
   - One lower-std/build cycle, with the fixpoint clean.
   - The fixed probe, the guard set, clippy, `pre-push-check` and `tests/compiler` at both settings.
   - Wasted: `where.nika` listed no tests. Also, `pre-push-check` printed nothing on success, so I ran it a second time to see the exit code.
   - Possibly not needed: the full `tests/compiler` run. `buffers.nika` is not on builder.md's `check/`/`emit/` list, but the brief's *Done when* asked for it in effect.
4. **Most useful change:** say in builder.md whether a change to a lowering pass outside `check/`/`emit/` (`buffers`, `views`, `tether`) needs the full `tests/compiler --both-settings` run, or whether the new test plus the guard set is enough. That decides whether a 10-minute run is required. A smaller fix: in the clippy line, write the scratch path out in full rather than relying on `$S` inside the subshell.
