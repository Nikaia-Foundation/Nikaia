1. **What cost the most:** the `drive.nika` rebase with `--autostash` in the shared checkout. It took my uncommitted files away mid-run and folded my Part III hunks into another agent's commit. Finding the saved copy (`.git/rebase-merge/autostash`, `c76cbd6f`), waiting for the rebase to end and re-applying my diff cost about 10 turns. After that came two red gate rounds of about 10 minutes each: fmt wanted `pub use crate::env` in sorted order, and clippy refused items after the test module in `hash.rs`.
2. **What I had to look up:**
   - A new `std` module needs a `pub use` in `lib.rs`'s `prelude`. Generated programs reach modules only through it, so without it rustc reports "cannot find module `env`".
   - Free functions take `String`, not `ref String`; `ref String` is refused outside `extern`.
   - `fs::Path` has no `is_dir`; the declared way is `fs::metadata(..).is_dir()`.
   - Part III's `touches` table is section 13.5.
   - `std` has no `as_bytes`, only `.bytes().collect()`.
3. **Checks I ran:**
   - Probes with `nikaia run` on a copy of the binary; the first `lower-std` and the fixpoint diff.
   - `cargo check --features compiler`; the gate three times; `nikaia-std --lib hash::`; the five `it` filters.
   - Wasted: two gate rounds that rebuilt and re-lowered only to fail at fmt and clippy. `cargo fmt --check` and the clippy of `nikaia-std` first would have cost under a minute.
4. **One change that would have saved the most:** builder.md should say "before the gate, run `cargo fmt --check` and `cargo clippy -p <crate> --all-targets -- -D warnings` on the crates you touched". The brief for a new `std` module should list its three places: `pub mod` in `lib.rs`, `pub use` in the `prelude`, and the `std.contracts` block. Separately, `drive.nika` should not rebase with `--autostash` while builders are working, or should at least leave the autostash in `git stash list`.
