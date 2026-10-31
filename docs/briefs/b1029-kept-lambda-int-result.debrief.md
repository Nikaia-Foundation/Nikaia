1. **Most turns: an unplanned lowering defect.** My new call in `check_core_leaves.nika` passed a `ref Ty?` from a block arm, and rustc refused the lowered code. About 15 turns of probes (q2–q10) narrowed it to `a_lent_part_is_handed_on_as_it_is`, which removes the argument from `lent_args` in block arms. Next: bootstrapping. The old binary lowered my own fix wrongly, so I had to build once without that line, put it back and lower again. Third: the brief's planned fix (the type in the cast) does not work in rustc. That cost one full build cycle before a 10-line rustc check showed it.
2. **Looked up:**
   - how facts reach the emitter (`self.facts`, no Rust adapter);
   - `integer_named` (in `check_names.nika`) and that `Ty::Fn.result` is `Ty?`;
   - `emit_flow_result`'s neighbours, and the `lent_args` / `views_handed_on` code in the checker and emitter.
   
   That `scripts/commit.nika` reads the attribution only from `<report minus .report.md>`, so a `.stepN.report.md` gets the wrong co-author and session. The brief could have named that, plus `integer_named`.
3. **Checks:**
   - Useful: the test program on the old binary (it confirmed the failure); small `nikaia run` probes; one plain `rustc` probe; the guard set, clippy and `pre-push-check`; `tests/compiler` at both settings (411 green).
   - Wasted: the first build with the cast-only fix, because rustc ignores an `as` cast's type and a 10-second rustc probe would have shown it first. Also a second build that failed because of the bootstrap problem.
4. **Change that saves the most:** add a rule to builder.md or the brief template: *"Before building an emitter fix, write the exact Rust you plan to emit into a plain `.rs` file and compile it with `rustc`."* That would have caught the wrong cast plan before any build. Second: in builder-facts, *"a fix whose own tools code needs the fix to lower: build once without the caller, then add it back"*, and that step reports must commit through the main report name.
