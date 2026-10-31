1. **What cost the most.** The fix took one edit and one build, so the overhead was in testing:
   - The scratch-project recipe in *Traps already met* has no `src/main.nika`, so my first `nikaia test --project` run died with "cannot read …/src". That was one wasted run.
   - I widened the related-test set by grepping for NK2202, NK1152 and `sync`. That pulled in two programs that need support files from `tests/compiler`, so they failed in the scratch project and cost an extra run on the old binary to clear them.
2. **What the brief could have handed me:**
   - That bare `sleep` is refused (NK1117) and the pausing call is `use std::time` with `time::sleep(1)`.
   - That probe 2's `comptime W: String` is refused by design (NK1167) and needs `ref String`.
   - That `tests/compiler` has no `sync_*` programs. The real related tests are the `it` modules `sync_of_a_lambda`, `comptime*` and `build_time`.
   - The brief's location for the cause (`a_named_call`) was exact. I also had to look up `callee_named` in `calls.nika` to see how a bracket call on a non-function value (`fs[0](x)`) would be treated.
3. **Checks.**
   - Useful: the probes on the old and new binaries, the build cycle, the fixpoint `cmp`, the two new tests with `--bless`, the same tests on the old binary, the `it` filters, fmt, clippy 1.97 and `pre-push-check.sh`.
   - Partly wasted: the broad 24-program scratch run, whose two failures had nothing to do with my change, and the failed bless run.
   - Partial, and my report should have said so: I ran clippy 1.99.1 on `nikaia-std` only, not on the whole workspace.
4. **The one change that would have saved the most.** Add `mkdir -p $D/src && printf 'fn main() {\n}\n' > $D/src/main.nika` to the one-program recipe in builder-facts. Second choice: the brief names exactly which related tests to run (`tests/compiler` programs and `it` modules) instead of patterns that match nothing.
