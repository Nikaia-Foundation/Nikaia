1. **What cost the most.** A false green. The run-3 report said `cargo check -p nikaia-std` was clean, so I ran the same command. It finished in 0.64 s and checked nothing, because `tools` only compiles with `--features compiler`. Finding that cost a `touch`, a re-run and a grep of `lib.rs`. My own probe slips cost two more `nikaia run` calls: I twice wrote a `??` fallback that gave `i64` where `i64?` was needed.

2. **What I looked up that the brief could have handed me.**
   * The `compiler` feature gate.
   * The notes disagreed on how to type a `let` lambda. Run 2 said type it on the `let`; the b997 report says leave it untyped so its first use types it, and that a typed one becomes `Kept`. I had to read the b997 report to settle it.
   * `Kept`'s definition in `func.rs`.
   * Whether any lowered file already used `+ Send + Sync>` (none did, so the `Sync` clash had never been hit before).

3. **Checks I ran.**
   * Useful: the patch check and apply, three `lower-std` runs (seconds each), `cargo check --features compiler` with hand edits to the lowered file to find defects 1 and 2, and the final `deep` probe, which proved defect 3.
   * Wasted: the `cargo check` without the feature, and the two broken probes.
   * I did no build, ran no `it` or `tests/compiler` tests, and did not run the gate, which was right since nothing was committed.

4. **One change that would have saved the most.** Add to `docs/builder-facts.md`, under *Traps*: "Check that lowered tools code compiles with `cargo check --profile fast -p nikaia-std --features compiler --message-format short`. Without the feature it checks nothing." For a move like this one, a brief should also tell the builder to run that check right after the first `lower-std`, before writing any Rust host. One more line in the brief would have warned of defect 3 in advance: "the Rust uses `&dyn Fn` because `computed` calls itself with a new closure; check the lowering keeps that".
