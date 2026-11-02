1. **Most time.** The two clippy rebuilds were the slowest steps, and the second one only showed that another agent's uncommitted `diagnostics/mod.rs` was failing. The commit script's refusal cost the most turns. It said "`sync.rs` is unchanged" when the real problem was my comma-separated `Files:` line. I misread that as a staging problem, unstaged the files, and reran the script.

2. **Lookups the brief could have handed me.**
   - The brief points to `.claude/agents/qa.md`, which I never opened. I worked from the task and the system rules.
   - `nikaia` is not on PATH, so I had to find `target/fast/nikaia`.
   - The `Files:` format (space-separated, no commas) was not stated. I only found it by grepping an earlier report.
   - How lowered test helpers are meant to be exported, `pub` or otherwise, was also not stated. I chose `pub` on my own and hand-edited the lowered `.rs` instead of regenerating it, because `lower-std` would have touched other agents' files.

3. **Checks.**
   - Useful: `cargo clippy --all-targets -- -D warnings`, `cargo test -p nikaia-std --features compiler --lib threads`, and `pre-push.nika`.
   - Wasted: `rustfmt --check` on a single file without `--edition 2024` gave a false let-chains error. I should have used `cargo fmt --check`.
   - Not run: I did not run `cargo fmt` on my edits.
   - Not checked: the CI log came from `ci/briefs`, and I never confirmed that it matched the current HEAD.

4. **One change.** Add a short "QA quick facts" block to the brief or `builder.md`:
   - The `nikaia` binary path.
   - The `Files:` line is space-separated.
   - The commit script's "unchanged" refusal usually means a malformed `Files:` line.
   - Run `cargo clippy` first, since the log's `error:` lines name the cause.
   - If a dirty file belongs to another agent, report it and do not wait on it.
