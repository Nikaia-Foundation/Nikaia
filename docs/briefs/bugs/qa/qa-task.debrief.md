1. **Time sink:** Finding the right input file and getting a working `nikaia` binary.
   - My first `ls` of `docs/briefs` dumped hundreds of lines, and the first read of `qa-input.txt` was mostly CI log preamble.
   - `nikaia` was not on PATH. A dev-profile `cargo build -p nikaia` hit a winnow recursion limit before I found that `--profile fast` works.
   - The cause itself was quick: a missing field, visible in the compile error and the working-tree diff.

2. **Lookups the brief could have handed me:**
   - Which lane's `qa-input.txt` applies. The brief names two, and I had to infer the bug lane from my own path.
   - How to run `nikaia`: `cargo build --profile fast -p nikaia`, then `target/fast/nikaia`.
   - The `Attribution:` trailer and the report fields `scripts/commit.nika` needs. I copied them from another report.
   - That uncommitted files in the working tree may be the builder's own forgotten files, not another agent's.

3. **Checks:**
   - Useful: `cargo build -p nikaia-std`, which showed the error gone, and `scripts/pre-push.nika`.
   - Wasted: the dev-profile `cargo build -p nikaia` and the bare `nikaia` calls.
   - Not done: a fresh build of the commit alone. I built with the whole working tree, so I did not show that HEAD plus my commit compiles without the other agents' edits. I never reran #1140's two new tests, and clippy and `cargo fmt` were not meaningfully run.

4. **One change:** Add a "Quick start" to `qa.md` with three lines:
   - Which `qa-input.txt` to read (the one beside the brief's lane), and to `grep` it for `error` first.
   - The binary path (`target/fast/nikaia`) and the build command.
   - To check `git status` for uncommitted files that the failing symbol needs, since a missed `Files:` entry is a common cause.
