1. **Time:** The `tests/compiler` run was the longest step, a full `nikaia test --project tests/compiler` for a script-only change. The run was short otherwise, about 8 tool calls. The edit itself took one call.

2. **Lookups:** Nothing beyond the brief. The line numbers and the `grep -l drive.nika` command were already in it. I read `docs/builder-facts.md` in full although it is mostly about self-hosting moves that this brief doesn't touch. The brief could have named the four test programs that mention `drive.nika`. It could also have said that `nikaia test` has no name filter, so only the whole project run is possible.

3. **Checks:**
   - **By hand:** a copy of the b998 report plus the real 40-commit log, run through `--rewrite`. It printed `Pushed: f2599c1c (pushed by drive.nika)`, which is the check that mattered.
   - **Project run:** `tests/compiler`, 356 passed. This covers the four drive tests, but it also ran 350 unrelated programs.
   - **Guard set:** the `it` filter set, 23 passed. This was the gate, and it was probably unnecessary for a script-only change, though the builder rules require it.
   - **Not done:** I did not run clippy. I also ran `pre-push-check.sh` through `tail -3`, which printed nothing, so I did not confirm its result. `commit.nika` then pushed without a failure.
   - **No failing-first test:** I added no new test, so I did not run one against the unfixed script. `drive_rejected_push` feeds the log in by file, so none of the existing tests could show the new `git log` call.

4. **Change:** The brief should say that the `git log` line itself cannot be reached by any existing test (`drive_commit` is driven by git state), and that the by-hand check against the real report is the intended verification. It should also say that a script-only change needs only the four named drive tests, not the whole project run. In `builder.md`, the gate should state which gate parts apply to a `scripts/*.nika`-only change.
