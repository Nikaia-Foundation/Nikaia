1. **What cost the most:** working out whether the red was real. `architect-input.txt` said only `RED`, and `suite.log` was 200 lines of repeated compile errors. I had to compare the binaries, check `gate.nika` with both of them, and do a dry run that failed on a flag I had guessed (`--dry`). Tracing #1065 cost about 8 turns. Two of those were wasted: I searched the wrong file for `emit_core_a_field_call`, which was in `emit_core_reads.nika`, and the `sed` then got an empty line number.
2. **What I had to look up:**
   - **How the driver runs the suite:** which binary runs `gate.nika` (a copy at `target/suite-nikaia`, made in `drive_launch`) and in which worktree. The input could have named both.
   - **CI state:** fetched with `gh api` and not needed in the end.
   - **The trace:** `check_core_call_kept` and `check_core_call_lent` as the model for the fix. `where.nika` gave the definition but not the parallel path, so I found it with grep.
   - **Formats:** the brief format and `common.part`. These were cheap.
3. **Checks run:**
   - **Useful:** the binary `cmp`, `check` of `gate.nika`, the probes for rows a, b and c, and the one-view and kept-call variants that narrowed the cause.
   - **Wasted:** the `gate.nika` dry run, the CI run listing, and the push attempt. The push could only fail, because HEAD held b12's untested commit, and I could have seen that from `git log origin/develop..HEAD` before trying.
4. **Change that would have saved the most:** a `gate-error` (the gate failing to compile, as opposed to a failing test) should get its own stop line, `GATE-ERROR`, with the first error and the sha of the suite binary. The driver should then re-run the suite once with a freshly built binary before it starts the architect. Separately, the architect task should say: "with unpushed builder commits below yours, do not push; the driver pushes".
