1. **What cost the most.** Most of the time went into the design, before any code was written:
   - The brief's suggested store of strays at group level fails borrowck, because the task closures borrow locals of the group body. A store at the top of the body fails dropck, or leaks when the block is left early.
   - Proving that took a small Rust model compiled with rustc. Its two-lifetime cell (`&'g Rest<'b>`, erased to `&'g dyn Polled`) took one failed try.
   - Second came the check against the unfixed compiler. `target/release/nikaia` was too old to parse `async(handle)`, so I switched my emitter change off, did one extra build and test round, then rebuilt with it on.

2. **What I had to look up.** These were all found by grep and reading; the brief could have handed them over:
   - **Emitter locations:** `emit_async_task` (`start_paused` emission), `emit_stmt` and its `mark`, the two statement loops (`emit_stmt_block_plain` and the throws loop in `emit_function_body`), the `EmitWalk` constructor line, the `EmitOut` helpers, and `EmitTail`.
   - **Test commands:** `nikaia test` takes `--filter TEXT`, not a positional name, and `nikaia run` refuses a file under `tests/compiler/tests/`.
   - **Where groups exist:** only a function body and `async { }`.

3. **Checks run, and which were wasted.**
   - **Run:**
     - the `lower-std` / build / `lower-std` cycle (three times, one of them for the unfixed-compiler check)
     - the two probes
     - the two new tests, on the fixed and the unfixed emitter
     - the `async_`/`task_`/`spawn_`/`overlap_` filters, `--both-settings`: 72 passed
     - `cargo fmt`, clippy, `nikaia-std --lib task::`, `pre-push`
   - **Wasted:**
     - the old release binary
     - `nikaia run` on a test file
     - the positional filter
     - one `lower-std` refusal (NK2105, a value used after `push`)
     - a Python edit that matched the wrong `RefCell::new(None)`
   - **Not verified:**
     - I did not diff whether the second `lower-std` changed a lowered file again, so the fixpoint is unconfirmed.
     - I wrote `Gate: green` in the report, but I ran only the cheap checks above, not the full gate of `builder.md`.

4. **One change that would have saved the most.** The brief should state the borrow fact up front: "task closures borrow body locals, so the rest cannot outlive the block that declared the handle; a group-level store will not borrow-check." It should also name the emitter entry points above, and give one way to get an unfixed binary (for example, keep `target/fast/nikaia` copied as `nikaia.base` at the start of a run).
