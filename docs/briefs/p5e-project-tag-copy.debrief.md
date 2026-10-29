1. **What cost the most:** the shared checkout, not the code. The one test run took about 10 minutes. Three of its failures came from another session rebuilding `target/fast` mid-run, so I spent a rerun and a dig to clear them. Clippy was red on another agent's uncommitted `view_changes.rs`, so I needed a second clippy run with that lint allowed. `lowered.rs` held a foreign hunk, so `commit.nika` couldn't be used: I built a patch of my hunk and committed by hand. Then the badge needed a throwaway worktree to tell my share change from theirs. The move itself took two `lower-std` refusals and one build.
2. **What the brief could have handed me:**
   - Text can't be cut at a fixed number (NK1214), and `Bytes.text()` can fail (NK2605); the answer is `s.scalars.take(n).collect()`.
   - `fs::write` is async, so `trace` lowers `async` and the adapter needs `block_on`.
   - A `String` passes where an `fs::Path` is expected.
   - `std`'s `IoError` prints different text from `std::io::Error`, which matters for the stop condition on changed messages.
   - Which adapter it meant: `refusal` (plain `anyhow`) or `refused` (a refusal diagnostic).
3. **Checks:**
   - Useful: `lower-std` with the old binary (found both refusals in seconds), `cargo check`, one build cycle, the filtered `it` run plus the guard set, `--lib project::`, the rerun of the three failures, `pre-push-check`.
   - Partly wasted: the first full-tree clippy (red on foreign code). The `where.nika` lookup found no tests for these functions, so the `it` filters were broad guesses.
4. **Change that would save the most:** builder.md should give a recipe for shared-tree collisions, by name:
   - commit a single hunk of `lowered.rs` (`git apply --cached`), or let `commit.nika` take a patch;
   - run clippy with a named foreign lint allowed and report it;
   - measure the badge on HEAD plus your own diff;
   - copy the test binary before the `it` run.

   Each of these cost me a detour this run.
