1. **What cost the most.** Finding out why a build-time value failed. NK1127 "couldn't build the program that computes it" gives no reason. I had to find the generated `main.rs` by hand. The brief points to `~/.cache/nikaia/`, but for a run with `--project` it sits in `<project>/target/nikaia/build-time/comptime/<hash>/`. When lowering refused the program, no file was written at all. Step 3 needed three build cycles, one per layer:
   - the dump of `T?`;
   - the Rust type spelling (`Option<&str>`);
   - the earlier `comptime`s the sub-program carries (a `Some` as a bare literal, `null` with no type).
2. **What I had to look up.** These five points were not in the brief:
   - the sub-program re-checks earlier `comptime`s as literals (`comptime_unit_without_its_constants`), so every new value shape must also survive as a carried literal;
   - `rust_constant_type` / `declared_below` skip `Ty::Nullable`;
   - a text `comptime` is declared `ref String`, which is `Named str view`;
   - `Ty::fits` exists;
   - the decoder already reads `(v Option …)`.
3. **Checks run, and which were wasted.**
   - Run: build cycles with the `lower-std` fixpoint, single-test runs with `one.sh`, and one batch of 54 tests with `--both-settings`, which was efficient.
   - Also run: clippy twice (stable and 1.99.1), `fmt`, `pre-push-check`, and `nikaia-std --lib` for the three modules.
   - Wasted: the `nikaia-std --lib` runs, which found 0 tests (the modules are lowered `.nika`), and clippy on local stable once 1.99.1 was installed.
   - Missing: the proof that the new tests fail on the unfixed compiler is old probes, because I kept no copy of the old binary.
4. **One change that would have saved the most.** In builder.md: "Copy `target/fast/nikaia` to your scratchpad before your first build, for the fails-unfixed run." Plus one line in the brief: "a new `comptime` value shape must also work as the literal the sub-program carries (`comptime_literal_of` → re-check); the generated `main.rs` is in `<project>/target/nikaia/build-time/comptime/`, and none is written when lowering refuses."
