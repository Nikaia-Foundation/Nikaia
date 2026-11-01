1. **Most turns and time.** Most of it went to the or-pattern split. I first found out that the "or-pattern split" was R261, a text pass in `main.nika` on the Rust source, not in the pattern writer the brief named. Removing it broke the corpus floors. Re-fencing it for `: Some(` took two build and corpus rounds. Separating the three rules into three commits cost about as much again. `commit.nika` adds whole files, so I had to build per-rule hunk patches and apply and reverse them by hand.

2. **What I looked up that the brief could have handed me.**
   - The tree writer is `block`, `if_let` and `match_expr` in `tr_stmt.nika`. `pat` is in `tr_types.nika`. Nodes have a `parent` field.
   - R261 lives in `main.nika` (`rs2nika_or_arms`). It runs on Rust text, and both engines share it.
   - Test `.args` files are `--profile check-free f.rs`. Older tests carry `--engine lines`, which I copied wrongly at first. A test is run from inside its `.in` directory.
   - `nikaia` is not on PATH, so I used `target/fast/nikaia build` and `test`.
   - The corpus layout: `tests/corpus.in/<name>/before.rs` and `after.nika`, floors in `manifest`.
   - The suite worktree has an old `rs2nika` binary for before/after diffs of drafts.

3. **Checks.**
   - Useful: the `nikaia test` run in the convert project, the corpus run, and diffing old and new drafts. Those diffs showed which rule moved the score.
   - Wasted: the first stdout files, written from the wrong invocation (`--file`, no `f.rs`), which made the test fail. A stray `sed` range that left a half function in the `some_field_pattern` input. A noisy `grep -l` over the tests directory.
   - Not run: `cargo fmt` and clippy (no Rust changed), and the pre-push check printed nothing.

4. **One change.** Add a `Facts` line to the brief: the `or-pattern split is R261, rs2nika_or_arms in main.nika, text pass shared by both engines; corpus floors in tests/corpus.in/manifest; test args are '--profile check-free f.rs'`. Also let `commit.nika` take `Files:` entries that stage a hunk range, or let the brief allow one commit when its rules share a file. That would have saved the most.
