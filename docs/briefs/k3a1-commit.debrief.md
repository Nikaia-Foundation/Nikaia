1. **Cost:** The two builds and `lower-std` runs, and the 112 s `it` run beside clippy. The run was otherwise short, about 8 tool calls. The second build and `lower-std` pass only confirmed the fixpoint, which came out unchanged.

2. **Lookups:** Nothing outside the brief and `builder-facts.md`. The brief gave the files, the commit subject and the Body source. The one thing I worked out myself was how to cut the `lowered.rs` sentence out of the Body: I used a sed on the report's `Body:` line. The brief could have said the stripped sentence's exact text, or that `lowered.rs` held only the one `comptime_tree` hunk.

3. **Checks:**
   - Run and useful: the `git diff --stat` and `git status`, the `lower-std` fixpoint, clippy, the `it` filters and the guard set, and the pre-push script.
   - Probably wasted: the first `lower-std` and build together. I should have rebuilt once and then run `lower-std` once.
   - Weak: I read the pre-push script through `tail -5`, which printed nothing, so I did not see its full output.
   - Not run: `tests/compiler`. The change touches no `check/`, `emit/` or `tools/check_*` or `tools/emit_*` files, so the gate does not require it.
   - Not verified: I never checked the commit message that `commit.nika` wrote. I also never ran `--dry-run`, and `Disk freed: 0` was my guess.

4. **Change:** Have `commit.nika` print or check the pre-push result, and have the brief say which `lowered.rs` hunks are expected. A one-line "expected `git status` after the fixpoint" in the brief would have let me skip the manual diff reading. A `tests/compiler` line in the gate would also be clearer when `comptime_run.rs` changes, since it is neither `check/` nor `emit/`.
