1. **What cost the most:** two avoidable rounds.
   - My first clippy and test run reported almost nothing back, because I chained `export` and `S=` with `&&` before the `&`, which pushed the whole chain into the background. I had to rerun.
   - Clippy then showed that four existing Rust `it` files call `Foreign::of`, `.library()` and `Foreign::default()`. The brief's caller list (`project.rs`, `modules.rs`) left out the tests, so I found them only after the build.
2. **What I looked up that the brief could have handed me:**
   - How tools files iterate a `BTreeMap` (a plain `for (k, v) in m`; there is no `.iter()`), and that `BTreeSet` has no `extend`.
   - That `fs::read` returns `Bytes` and is async, so the moved function needs `block_on` on the Rust side.
   - That `read_dir` returns sorted names, not paths.
   - That `Ledger::blank` is already in Nikaia (`ty.nika`).
   - Whether two generic host parameters (`[M: A, H: B]`) lower; I wrote a probe for it.
   - The signatures of `crate_root` and `read_beside`.
3. **Checks run:** two `lower-std` runs to fix refusals, `cargo check`, one fast build, a fixpoint run, the brief's `it` filters plus the guard set, the four adapted test modules, `--lib project::`, `nikaia test --project tests/compiler`, clippy, `pre-push-check` and the score.
   - **Wasted:** the first clippy and test run (the shell mistake above) and `pre-push-check` before `badges.py`, although builder-facts already warns about that badge.
   - **Missing:** the commit script seems to have dropped my `Rs2nika:` line from the report, which now ends at `Disk freed:`. The figure was 21 of 79 lines kept, under 60 %. The supervisor should add it back.
4. **One change that would have saved the most:** have the brief's *Callers* list grep `crates/nikaia/tests/it/` as well. Also add one line to builder-facts listing what tools code can do with collections: maps and sets are walked with `for`, there are no `.iter()` chains on them, and `Bytes` is copied into a `Vec[u8]` with a loop. That line would also have told the converter not to write those chains in its draft.
