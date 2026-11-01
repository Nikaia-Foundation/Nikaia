1. **Most time:** the field-path case, `bag.items.drain()` on a `let mut` struct. My first fix (treat any field path as emptied in place) was too broad. It rewrote `node_numbers.rs` into rustc-invalid Rust, and that broke the build. I couldn't regenerate with the broken binary, so I had to `git checkout` that one generated file, rebuild and run `lower-std` again. The other cost was `ledger_items_stored`: its parameter is lowered as `&[..]`, so `mut codes` plus `codes.drain()` was needed, and then the Rust caller had to pass `&mut`.

2. **Lookups the brief could have handed me:**
   - The checker half of `drain` lives in `check_core_call_on.nika` (~:984), and the `map_drains` and `kept_drains` facts sit beside it.
   - The emitter half is at `emit_core_calls.nika` :981.
   - Draining a field path of a local is not "kept" today, so it becomes `into_iter()` and moves the field.
   - Vec field drains in `node_numbers.nika` depend on that partial-move behaviour.
   - `ledger_items_stored` must take `mut codes` to drain it, and the `contracts/mod.rs` caller has to change with it.

3. **Checks:**
   - The unfixed-binary run showed NK1171 for all three methods.
   - The new test passed with `--both-settings`.
   - The fixpoint was checked, along with the `contracts::` lib tests, `it throws ledger contract` (144 tests), clippy and pre-push.
   - Wasted: two build and `lower-std` cycles around the bad `node_numbers` output. The `ledger_items` mismatch could have been seen from one `lower-std` before building.
   - I didn't do the separate fails-before stash run. The old binary's NK1171 stood in for it.

4. **One change:** add a line to the brief or `builder.md`. It would say that when a checker change alters how `drain` is lowered, I should run `lower-std` and diff `lowered/` for files I didn't touch, such as `node_numbers.rs`, before building. It would also say that a broken lowered file is restored by name with `git checkout`, since the broken binary can't regenerate it. The brief should also state that a field path of a local is not covered by the existing kept-drain logic.
