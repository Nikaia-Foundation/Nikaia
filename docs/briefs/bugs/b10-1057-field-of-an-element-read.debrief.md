1. **What cost the most time:** the full `nikaia test --both-settings --project tests/compiler` run, which took about 10+ minutes for 463 programs because the tool has no name filter. The fix itself took one edit and one build cycle, since the brief's trace was exact. On turns, the map row cost two extra probes: first `use collections` (the right spelling is `use std::collections`), then `BTreeMap::new()` (refused with NK1149; the right form is `collections::BTreeMap()`).
2. **What I had to look up:**
   - The two map spellings above. The brief could have given the map probe as a complete program, as it did for row a.
   - How to make a `.stderr` for one test. `nikaia check` inside `tests/compiler` checks the whole project, so I ran the copy in the scratchpad and added the `tests/` prefix to the path by hand.
   - Where Part III's NK1131 text is. The spec line is 52 KB, so my grep printed far too much the first time.
3. **Checks:**
   - Ran: rows a, map, d and e on the old binary and on the new one; the cycle (`lower-std`, build, `lower-std`); a second build to confirm the fixpoint (a no-op); the test project; fmt, clippy and `pre-push-check.sh`.
   - Nothing was clearly wasted. But the long run only needed to cover four tests by name, so a name filter would have made almost all of it unnecessary.
   - The "fails on the unfixed compiler" evidence came from the old-binary probes of rows a and map. I did not run the test file itself against an unfixed build, which saved a rebuild.
4. **The change that would save the most:**
   - Give `nikaia test` a name filter, or have builder.md give one recipe for writing a single test's `.stderr` (run a scratch copy, then fix the path prefix).
   - In the brief, write every probe row out as a full program.
