1. **Most time:** the test runs, not the work. The filtered `it` run took about 14 minutes and failed one test, `drive_kept`, probably because it ran beside clippy. Rerunning that one test cost about 10 more minutes. My two wait loops hit their timeouts and cost turns. The move itself was already in the tree and needed no edits.
2. **What I looked up:**
   - The first run left no converter draft behind. I had to rebuild the moved Rust from `git show HEAD:…project.rs`, finding the line ranges and doc-comment starts myself, and run `rs2nika` again.
   - I searched the old scratchpad for the draft (one wasted `find`).
   - Whether `Seq.chain` exists in std (it doesn't, NK1171): one `lower-std` try.
   - The brief could have handed me the cut ranges at HEAD, or the path of a saved draft.
3. **Checks:**
   - Useful: `cargo check`, the build cycle (fixpoint), `--lib project::`, the compiler package `--both-settings`, the filtered `it` run with the guard set, clippy, and `pre-push-check` (it caught the badge).
   - Wasted:
     - The `it` filter `package` also ran `nikaia_test::the_compiler_test_package_passes_at_both_settings`. That test repeats `nikaia test --both-settings`, which I had already run directly, and it flaked.
     - The lone rerun of that test spent another 10 minutes confirming it.
     - The first `cargo check` was redundant, since the cycle builds anyway.
4. **One change:** a restart brief should name the saved draft (`draft.nika` plus the cut `.rs`) beside the partial diff, so `Rs2nika:` needs no re-extraction. builder.md should say that `it` filters must not match `nikaia_test::the_compiler_test_package_*`, since that test duplicates the direct `--both-settings` run. Exclude it, or use `package_` and `dependenc` as exact module filters.
