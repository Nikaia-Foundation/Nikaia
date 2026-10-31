1. **Most turns: finding the code paths.** Before the context reset I spent most of the run reading the checker and the emitter: where a parameter is declared, where a call is checked and named, how `T::fields` is unrolled, how a copy is named and emitted, and where moves are decided. `check_core_pilot.nika` and the `emit_*` files are large, and reading them filled my context until it reset. Two smaller costs came after that:
   - One build cycle went to a rustc inference error in the lowered `(bound ?? on).clone()`.
   - Some turns went to `pre-push-check.sh` failing on ADR-384, which has nothing to do with my change.
2. **What the brief could have handed me:**
   - The emitter side: where a specialised copy's name is made (`emit_function.nika`, `walk.specialising`), and where a call to a copy is rewritten (`emit_core_args.nika`, `emit_call_special`).
   - Where the move check is decided: `hands_over_read_at` in `check_arm_leaves.nika`.
   - Where declared types are collected, so I knew where to add NK1320: `collect_declared`, `check_function_frame`, and the typed `let`.
   - That `comptime` value parameters do not parse at all. The brief said `comptime f: meta::Field` "means the same", and that turned out to be impossible inside its allowed files.
3. **Checks:**
   - Useful: the old-binary baseline on both tests, the build and `lower-std` fixpoint cycle, the 41 regression programs with `--both-settings`, the NK1320 placement probe, fmt and clippy.
   - Mostly wasted: the `nikaia-std` `tools::` unit tests (11 tests, unrelated to this change); the second fixpoint build, since the diff stat was already the same; the `bash -x` trace of `pre-push-check.sh`.
4. **One change:** the brief's *Locations* should name the emitter and move-analysis functions as well as the checker's (one line each, from `scripts/where.nika`). It should also say up front that a `comptime` parameter does not parse yet. That would have saved most of the reading that used up the first context. In `builder.md`, a known-red pre-push step (like ADR-384) belongs under *Red on develop now*.
