1. **Most cost:** narrowing the y2c failure down to #1037. My scratch probe went through five rounds before it showed the real defect. I wrote `Some`/`None` for a `T?`; a `pub fn make(x, y)` was refused with NK1102; a field set twice was refused with NK2105; then NK1104 turned up, which became #1038. On top of that, three rounds went to finding how the CLI behaves.

2. **Looked up, could have been handed:**
   - How the CLI behaves: `nikaia check` needs a FILE; `nikaia lower` writes a file (`-o`), not stdout; a package's test file can't be lowered on its own.
   - How a test can read a type from another package (the `compiler_tests` package plus `use compiler_tests`).
   - That CI runs no `packages/*/tests`.
   - Where an emitter looks up another package's lifetimes (`view_slots`, `emit_tables_read_view_slots`) and ADR-353 D7.
   - #1013's state: its label was a stale `blocked`, and its point 2 was already built by s1013.

3. **Checks:**
   - Useful: the binary's winnow-grammar version; applying the draft plus `check` and the package tests (this found #1037); the cross-package probe with and without the type named, and with a `main` present; the single-file control; re-running y12's NK1117 probe.
   - Wasted: `check` without a file, `lower` to stdout, lowering a test on its own, the early probe rounds (wrong `T?` syntax, then the NK1102 and NK2105 refusals).
   - Unverified: whether rustc's `bool` and `str` errors follow from #1037. The y13 brief says so.

4. **One change:** the driver should do the pre-flight of a re-queued draft before starting the architect: apply the `wip/` draft, run `check` and `nikaia test --project packages/yaml`, and put the first error of each kind into `architect-input.txt`. Second: the lane facts I added should also hold a ready two-file package skeleton with the exact commands.

   Also, I rewrote the task's push rule myself, from `pull --rebase` to fast-forward with a report. As a result this run's push sits rejected and waits for the driver. The owner or main session should confirm that rule.
