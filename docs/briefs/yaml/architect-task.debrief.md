1. **Most turns:** writing the `load` probe in a scratch copy of the package. I lost about five runs to syntax trial and error: `mut` goes before the parameter name, not on the type; the map type is `collections::BTreeMap`, not `std::collections::HashMap`; a named argument comes after `;`. I also put `.clone()` on `n.anchor`, which caused an NK1106 cascade I first took for the defect. Each package test run takes minutes.
2. **What I had to look up:**
   - Probe syntax: `mut` parameters, collection types and constructors, named arguments, legal text cuts (NK1214 refused `input[0..3]`).
   - Where NK2106 is raised: `check_takes.nika` and `check_values.nika`.
   - That `scripts/where.nika` runs as `target/fast/nikaia run`, since there is no `nikaia` on PATH.
   - That y21, the newest brief, had a cut-short rules block, so it was the wrong place to copy one from.
3. **Checks:**
   - Useful: reading the reports, input and suite log; the two-file minimal probes (A, B), which pinned the defect in seconds; cutting the package probe down to a copy without the map; checking the y19 rules block.
   - Wasted: the first three package runs, which stopped on my own syntax errors; `nikaia check src/main.nika` on a package file, the wrong mode, which gave misleading errors; the `gh` issue search for NK2106, which found nothing relevant.
   - Not done: a re-check of the NK2302 neighbour (`words_of`). I left it out rather than file a guess.
4. **One change:** add a "probe kit" to the lane facts in `architect-task.md`. It should hold:
   - a 10-line syntax crib (`mut x: T` parameters, `collections::BTreeMap[K, V]()`, `f(a; name: v)`, `x?.m()`);
   - the rule "probe the smallest standalone `.nika` first, a package copy only to confirm; `nikaia test --project`, never `check` on a package file";
   - `target/fast/nikaia run scripts/where.nika -- NKxxxx` to find where a code is raised.

   That would have saved roughly 8–10 turns.
