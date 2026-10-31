1. **Most time:** the gate's whole-package `tests/compiler` run at both settings (it ran the whole package because a `check_*.nika` changed). Second came one extra build of the unfixed binary, needed to show the new test fails there: the previous builder's binary already had the fix baked in, even though the tree was clean. The reduction itself was cheap, because the stopped builder's test program already reproduced it.
2. **Looked up:**
   - That `check_test` on a file inside `tests/compiler` checks the whole project. I wrote the `.stderr` from a scratch copy instead.
   - How an optional prints in `Ty` (as `T?`).
   - The name of the plain-call NK2105 function, to verify `Found:`.
   - What an ordinary call does when it reuses `t`. That decided refuse-versus-lend, and the brief could have stated it: NK2105, "Nikaia never copies behind your back".
3. **Checks run:**
   - Unfixed versus fixed probes: the stopped builder's program, two variants, a `t?.clone()` case and a plain call. The variant with a label the callee doesn't keep was the useful one: it showed the cause is "kept", not "optional".
   - One build plus one rebuild cycle, the gate, and the `yaml` package gate.
   - Wasted: the in-project `check` call (wrong output), and two greps for the gate's per-test lines, which it doesn't print.
4. **Change that would have saved the most:** when a brief hands over a stopped builder's partial work, it should say whether `target/fast/nikaia` was built with that work in it, and it should keep an unfixed binary next to the saved diff. That would have saved a build, and the brief wouldn't have needed to ask for "failed on the unfixed binary" to be proven again. builder.md should also note that `scripts/gate.nika` prints only per-step lines, so a builder doesn't hunt for test names in its output.
