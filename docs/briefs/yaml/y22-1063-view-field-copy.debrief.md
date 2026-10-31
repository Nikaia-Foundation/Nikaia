1. **What cost the most:** finding where the refusal came from. The brief pointed at `check_takes.nika` and `views.nika`, but the cause was in `text_tiers`: reading a field whose kind wasn't decided yet counted as text of its own. Running `nikaia check --tethers` on Probe A showed "view or text of its own, per value" at once. Before that I spent about 6 turns reading the checker. The second cost was Probe C: `NK1127` hid a rustc error in the build-time program. I had to find the generated `target/nikaia/build-time/comptime/*/main.rs` by hand to see that the lowering tied the result to the reference.
2. **What the brief could have handed me:**
   - That `--tethers` names the field's tier.
   - That `tier_at` treats an undecided position as owned.
   - That `NK1127`'s "couldn't build the program" means reading the comptime `main.rs` in the project's `target/nikaia/build-time/`.
   - That `source_nested` in `emit_function.nika` (from #1000) is where a parameter's lifetime is chosen.
   - That `tests/compiler/src/main.nika` already has `Places`, a struct shaped like `yaml`'s `Node`.
3. **Checks run:**
   - Run: the probes on the old and new binary, three `lower-std`/build cycles, `tests/compiler --both-settings`, the `yaml` package tests, `fmt::every_nika_file`, clippy, `cargo fmt`, `pre-push-check`.
   - Wasted: one build cycle, because my first emitter edit also narrowed the different-name case; `check_lookups.rs` changed and I had to restore it. Leaving the new test's `.stdout` empty turned one result of the long suite run red. I then filled the file from that output and did not run the test again, so it has not been seen to pass. I also never ran the three new test files against the unfixed compiler, only the probes they were made from.
   - The report says `Disk freed: 0`, but I did delete the scratch package's `target/` (small).
4. **Change with the biggest saving:** in builder-facts' "A wrong lowering: look before you read code" trap, add: "a NK2106/NK1106 on a text field: run `--tethers` and check the field's tier first (`text_tiers`); an NK1127 'couldn't build': read `target/nikaia/build-time/comptime/<newest>/main.rs` and compile it". Also have briefs give the exact output a new test's `.stdout` needs, rather than leaving it to be filled from the full suite run.
