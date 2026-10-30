1. **What cost the most:** checking the design against Rust lifetimes, not building anything. The `Kept`/`'static` mismatch only showed once I read the lowered trait next to `compute`'s callers. I then spent about three turns confirming in ADR-277 D11/D14 that no form lets a trait method lend a `fn` parameter. Turns were few overall, because I stopped as soon as the block was clear.
2. **What the brief could have handed me:**
   - A body-less trait method's `fn` parameter lowers as `Kept<dyn Fn + Send + Sync>`, which has to be `'static`. A Rust host therefore needs owned callbacks.
   - The callers pass closures that borrow their maps (`check/host.rs` `compute_comptime`).
   - Recursive free functions lower `async` (`Box::pin`) unless they are marked `sync`.
   - The run-2 patch predates its own notes: it still has `not_here: fn(String)` and an untyped `let known`.
3. **Checks run:**
   - Two `lower-std` runs and one `cargo check -p nikaia-std`. All were useful: they showed the `async` lowering, then a clean compile.
   - A `git apply --check` on the saved patch. Useful.
   - The disk check. Needed for the report.
   - None was wasted. I ran no build and no tests, because the block came before step 3.
4. **The change that would save the most:** probe the Rust adapter end to end before writing the brief, not just the Nikaia side. One small program would have shown the block in minutes: a Rust impl of a host trait with a `fn` parameter, called from a function holding a borrowed `&dyn Fn`. Both run 2 and this run stopped at the boundary between the host trait and Rust, and each time the brief had probed only one side of it. The brief should also settle the callers question up front: allow `Kept` closures in `check/host.rs` and `contracts/mod.rs`, or name the form that is missing.
