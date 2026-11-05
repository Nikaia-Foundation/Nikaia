1. **Most turns and time:**
   - **Four "failed" tests.** These tests start the compiler at the relative path `target/fast/nikaia`, so they fail when the suite runs from a scratch copy of the binary. At first I blamed this on another session rebuilding the binary and re-ran them, again from the copy, so that run was wasted. The only real fix was running them with `target/fast/nikaia` itself.
   - **One slow search.** A grep over all of `tests/`, `crates/nikaia/tests` and `docs` passed the 120 s limit and went to the background.
   - The build cycle itself was cheap: one cycle, no refusals.

2. **What I looked up that the brief could have handed me:**
   - Where `Eq` and `Ord` are wired as language traits: `STANDARD_BOUNDS` in `types.nika`, `compare_standard_bounds` in `check_compare.nika`, `check_core_call_on_undescribed` in `check_core_call_on.nika`, and the NK1126 text in `check_decls.nika`.
   - That bounds lower to Rust as written, so step 2 needed no change.
   - Which types have no `.clone()`: `TaskHandle`, `ChildHandle`, `Locked`, `SharedMut`.

3. **Checks:**
   - **Useful:** probes on the old binary, one `lower-std`/build cycle, a checksum fixpoint check, the filtered `tests/compiler` run, the five `it` modules, the `EXPECTED.txt` diff, fmt, clippy and pre-push.
   - **Wasted:** the re-run of the four tests from the scratch copy, and the slow repo-wide grep (a narrower one did the job in seconds).

4. **One change that would save the most:** in `builder-facts.md`, under *Test with your own copy of the binary*, state that tests which spawn `target/fast/nikaia` fail from any copy (the path is relative), not only when the binary vanishes. Name them (`a_trait_column_*`, `a_library_trait_with_no_impl_runs`, `bounds_expectation_guards_a_proof`) and say to run them with `target/fast/nikaia` directly. For language-trait briefs, the brief should also list the `Eq`/`Ord` wiring points above.
