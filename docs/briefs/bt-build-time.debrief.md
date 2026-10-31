1. **What cost the most.** The shared checkout. Other sessions rebuilt or deleted `target/fast/nikaia` while my tests ran, so I made binary copies. One copy picked up another agent's half-finished lowered files and gave 37 false failures. After that I re-ran the touched tests, ran the gate again, and sorted each failure into vanished binary, flaky NK2302 or real. That took more time than the move itself. The step-5 lowering defect came second: probing it down to 12 lines took several rounds.
2. **What the brief could have handed me.**
   - That deleting `a_wall` leaves `check/mod.rs` with an unused import, a file outside the brief's list.
   - That the badge moves (82.4 → 82.5) and `badges.py` must run before pre-push.
   - Which `rs2nika.names` `[types]` rows the new types need.
   - That `grammar` is a reserved Nikaia word, which is why the field is `outer`. I worked that out rather than reading it.
   - Whether the `??` chain in step 5 lowers cleanly. One check before handing out the brief would have caught the defect.
3. **Checks.**
   - Useful: the `it` filters (comptime, grammar_at_build_time, dsl_grammar, build_time), `tests/build-time` on its own, clippy and the guards inside the gate, and the probe for step 5.
   - Wasted: the first `tests/compiler` run on the 11:05 binary copy (stale lowered files from another agent), and the gate's `test` step, which died when the binary vanished. A `--keep-going` gate also repeats work I had already checked by hand.
4. **The change that would save most.** Let each builder build its own binary. The gate and the tests should run on that binary (or a pinned copy), not the shared `target/fast/nikaia`. `gate.nika --binary PATH` would do it. If that is not possible, builder.md should say to copy the binary right after the build and point every test run at the copy. Close behind: a brief should only include a one-line expression whose lowering someone has already checked. Otherwise it should say "may hit a lowering defect; stop at once".
