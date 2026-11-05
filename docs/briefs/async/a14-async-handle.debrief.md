1. **What cost the most.**
   - Before the context summary: tracing the async pipeline and applying a12's patch hunk by hunk.
   - After it: the test plumbing more than the compiler work.
     - `nikaia run` on a single file under `tests/compiler` is refused, and `nikaia test` takes `--filter`, not a bare name.
     - The exact `.stderr` text took three tries: `check` inside the project checks the project, and a scratch copy needs its path rewritten.
     - `commit.nika` refused my staged `git rm` twice.

2. **What the brief could have handed me.**
   - A table of `CheckAsyncWaitAt` → emitter kind → runtime call: `GroupEnd` → `spawn_here`/`spawn_paused_here`, a used handle → `start_here`/`start_paused`.
   - That `ChildHandle`'s Drop cancels the held rest of a task that pauses. That one fact decides how a live handle must be lowered.
   - That a `ChildHandle` type can't be written in a variable annotation (the Rust type needs `<'g,T,E>`).
   - Which places count as "stored in a value that outlives it". I picked a conservative set: assigned, or passed to a call.

3. **Checks.**
   - Useful: the old-binary `lower-std`, one build plus a fixpoint build (a no-op), probe programs on the new binary, the four `tests/compiler` filters, the five new tests on the old binary, the `it` tests `parser::` and `async`, clippy, `cargo fmt`, pre-push.
   - Wasted: the `nikaia run` attempts on test files, `nikaia check`/`lower` inside `tests/compiler`, the empty `--lib parser::` run, and the first two `commit.nika` attempts.

4. **The one change.** In builder.md, under *A new regression test*, add:
   - "Write a `.stderr` with `nikaia check` on a scratch copy, then rewrite `--> NAME` to `--> tests/NAME`."
   - "To delete a test, delete the file without `git rm` and list the path in `Files:`."

   Two lines that would have saved about eight turns.
