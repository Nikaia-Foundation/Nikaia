Nothing is committed. The run ended before clippy, the commit, the report and the disk cleanup. The fix is built and both new tests pass when I run their copies by hand. The second test also fails on HEAD's lowered files, as it should.

1. **What cost the most.** A compiler defect blocked the step. On a `drain()` loop element, `let mut x = item.field` lowered to `&item.field`, so rustc rejected my generated `modules.rs`. Narrowing it with four probe programs and fixing `check_core_stmt.nika` took a build and a hand patch to get past the stale lowered file. After that, wording the test program took several runs: `|x|` closures are not Nikaia syntax, and a separate lifetime defect lowered a function that returns a local view of a parameter with a `&'static str` result. I wrote the test around that second defect instead of fixing or filing it.
2. **What the brief could have handed me.**
   - The issue text (it was missing).
   - That test programs live in `tests/compiler/tests/`, not `tests/compiler/` as the brief says.
   - That they cannot be run in place with `nikaia run`, and that `nikaia test` has no filter for one program.
   - The `root()` idiom for reading repository files from a test.
   - Where lower-std turns test blocks into functions (`sysroot.rs`, `with_tests_as_functions`).
   - The other file with the same hand fix (`manifest.nika`).
   - Syntax facts: no `loop` keyword, closures are written `fn(l) { … }`, and `for` over an owned vector only borrows.
3. **Checks.**
   - Run: the lower-std / build / lower-std cycle (the fixpoint is clean), grep checks of the four lowered files, the old/new probe comparison and both tests against the old state.
   - Wasted: two `nikaia run` calls on files inside the project, which refuses them, and three edit-and-run rounds on the test's helper.
   - Not run: clippy, `cargo fmt`, the `sysroot::` and `nikaia-std` unit tests, `pre-push`, and `nikaia test --both-settings`. The two `.stdout` files were written from my own runs; neither was checked by `nikaia test`.
4. **One change that would have saved the most.** Add a short *Running one test program* recipe to `builder-facts.md`: where the file goes, how to run a single one (copy it to a scratch folder and `nikaia run` it, or a filter flag), and how to show it failing on the old state. Also fix the brief's test path, and add the issue's probe to the brief.
