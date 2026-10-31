1. **Most turns and time:** trying to reproduce b1046's `Found:` myself. I started with a copied probe package (`target/arch-probe`), which `nikaia test` and `check` didn't support the way I needed. Then I ran the whole `tests/compiler` package at both settings in the background. It was still far from done when I killed it, and it settled only one of the three tests. Second came the #922 research: whether `sha2` is a dependency of `std` (it isn't), and what a missing `touches` line means in Part III.
2. **Lookups the brief could have handed me:**
   - The plan rows' blocker status: which rows wait on #920 or #922, and that BT waits on nothing.
   - That `nikaia-std` has no `sha2` (its `Cargo.toml` :116, ADR-002 D4).
   - The `touches` rule (Part III :332).
   - The `process::run` signature, and that no tools file calls it yet.
   - The four stand-in `sha256_hex` methods in the tools files.
   - `docs/briefs/s3-fs-canonical.md` as the model for a `std`-entry brief.
   - The bug lane's naming scheme (`b<N>-<issue>`) and its next free number.
3. **Checks:**
   - Useful: `where.nika` and outline reads, the converter draft for BT and GR, the `std.contracts` greps, `gh api` on the issue labels, `git diff --cached --stat` before the commit.
   - Wasted: the `arch-probe` copy; the background both-settings run, which mostly ran unrelated tests and never reached two of the three; one failed `git add` on a path `git mv` had already staged.
4. **The change that would save the most:** a rule in the architect task that the architect does not reproduce a builder's `Found:` itself. It files the `Found:` as it stands, in brief form for the bug lane, and reproducing it becomes that brief's first step. The builder's report should also give the binary's sha and the exact command it ran. That alone would have saved a third of my turns.
