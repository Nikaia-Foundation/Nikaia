1. **What cost the most:** about 6 turns on step 1, the `rs2nika.names` rows. Re-converting with them changed nothing, so I read the converter's loader and ran a probe. It turned out the binary applies no `[forms]` rows at all, not even the global `{x}.span.at()` row. The second cost was waiting about 5 minutes for the `it` run and about 6 minutes for `tests/compiler`. I ran them in parallel, so that time was mostly unavoidable.
2. **What I looked up that the brief could have given me:**
   - Whether `print`/`println` lower inside a tools file. They do, as plain macros (one probe).
   - Whether `retain`, tuple-pattern closures or `?? continue` exist in `tools/*.nika`. Tuple closures appear nowhere in tools, but they lower.
   - That `SignatureOps::arguments()` skips a leading `self`. The brief says it "is `params`", which is not quite true.
   - The Rust signatures of `check_against`, `foreign::check` and `render_*`, and that `diagnostics` is `diagnostics/mod.rs`.
   - The plan row format, and where the `clippy::` allow list sits in `nikaia-std/src/lib.rs`.
3. **Checks run:** `cargo check --release`, one `lower-std` → build → `lower-std` cycle, the `it` filters plus the guard set, `--lib project::`, `tests/compiler --both-settings`, clippy twice, `pre-push-check` and `--score`.
   - **Wasted:** the first clippy run, which failed on `map_clone` in the lowered code. Running the converter twice (`--file` and the tree engine) only confirmed the dead `[forms]` rows.
   - **Too broad:** the `check` filter matches most of `it` (765 tests). That was fine this time, but it is wider than this change needed.
4. **One change that would save the most:** builder.md should state that `.map(fn(x) { x.clone() })` on a struct `Vec` trips `clippy::map_clone`, and that the fix is an entry in the tools allow list. Also, a brief whose step 1 depends on `[forms]` should first confirm that the current converter binary applies them; the driver could check with a one-line probe. That would have saved the clippy round and the converter investigation.
