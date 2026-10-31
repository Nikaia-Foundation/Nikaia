1. **What cost the most:** I misread NK1127. `nikaia check main.nika` showed it on `default_limits()`, and I wrote it off as noise from checking one file. It actually meant the build-time program no longer compiled. That cost one full package run with `--bless`, about 4 minutes, which wrote a `.stderr` for all 18 tests and left `load.stdout` empty. Getting the view checker to accept `resolved` cost the second most: `ref mut` params are refused, and so is storing views from a `ref Node` into a map.
2. **What I had to look up that the brief could have given:**
   - How a resolver that holds views is meant to be written: methods with `ref mut self` that take the tree by value (`Node`, through `drain()`), as `examples/1brc.nika` does. I found this only after NK1158 and NK2302.
   - That a map keyed by views is written `collections::HashMap[ref String, T]`, and that it needs `use std::collections`.
   - That `.clone()` of a view `String` gives owned text. Writing `n.anchor?.clone()` turned `Node.anchor` owned all through the grammar (NK1106 at lines 522 and 549).
3. **Checks:**
   - Useful: `nikaia check` on the file (fast, caught the view errors); `nikaia run` on a scratch copy with the defaults taken out, which showed the real rustc error; two small probes that narrowed the defect to a local bound from `m.get`; one scratch package run that verified the test output.
   - Wasted: the first `nikaia test --bless` on the real package, run while NK1127 was still unexplained, plus the cleanup of the `.stderr` files it wrote.
   - Not run: clippy, fmt, the `every_nika_file` test and `pre-push-check`, since nothing was committed.
4. **One change that would have saved the most:** add this to builder.md's traps:
   - An NK1127 on a default (`= f()`) after your change means rustc failed on the lowered file.
   - To see the real error, run `nikaia run` on a scratch copy with the defaults taken out.
   - Never `--bless` before `nikaia check` is clean, because a failed build blesses a `.stderr` for every test.

   Also add a line to the brief that the resolver keeps its state in a struct with `ref mut self` methods and takes the parsed tree by value.

I also created the local branch `wip/y27-load`, which the brief did not ask for, so the work is not lost. The owner may want to know that a branch exists.
