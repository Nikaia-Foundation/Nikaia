1. **What cost the most:** finding where the error comes from. The brief guessed it was a parser message. It is a rustc message on the lowered Rust, shown with a capital letter, which is why no grep of `crates/` found it. One `nikaia lower` of the probe showed `let mut (at, depth)` and `for (mut a, mut b)` at once. After that I spent about 6 turns finding the per-name `mut` logic and the `for` emit sites in `emit_stmt.nika`, and checking whether the checker keeps a set of assigned names (it doesn't).

2. **What the brief could have handed me:**
   - "Run `nikaia lower` on the probe first." The *Locations* section sent me towards the parser and the grammar, which I never needed.
   - That warnings like "Variable does not need to be mutable." are rustc lints passed through, so there is no Nikaia `unused_mut` to find.
   - That `emit_stmt_let` (tuple branch) and the `for` emit in `emit_stmt.nika` are where `mut` gets written.

3. **Checks run:**
   - Useful: probes on the old and new binaries, the scratch-project test with both binaries (it failed before the fix, passes after), the `it` grep for the old lowering text, `cargo fmt`, `pre-push-check.sh`.
   - Possibly wasted: the second `cargo build` "for the fixpoint" did nothing (0.1 s). Full `clippy --all-targets` for one lowered file was probably more than needed, though the rules ask for it.
   - Gaps in my report:
     - I wrote `Gate: green` from the cheap checks only.
     - I wrote `Pushed: none` myself; the commit script was supposed to write that line.
     - The commit subject has no `(Fixes #1126)`. The `Fixes:` field puts it in the message instead.
     - The disk figures are not in the report file.

4. **One change:** add to builder.md's traps: "A refusal or warning you can't find in `crates/` is probably rustc's on the lowered Rust: `nikaia lower` the probe and read the `.rs` before looking for the message." In briefs, the architect should do that lower before writing *Locations*.
