1. **What cost the most:** tracing how much build-time evaluation of a reflected field works today. It took about eight probe runs: the attribute syntax failed first, then an empty attribute struct, then the `T::fields` bound (`NK1171`), then `String?` not crossing, then `return f.name` (`NK1104`). Each needed a new scratch file. About 40% of my turns went there, and I needed it to see that j6 was blocked and that j5 had to come first.
2. **What the brief could have handed me:**
   - The exact attribute syntax: positional `@Name("x")`, and the `@meta::Attribute(field)` mark.
   - That `T::fields` is only reachable through a bound type parameter.
   - j3's test file as a working pattern. I found it only after a failed probe.
   - Where `NK1127` is raised (`check_core_comptime.nika`, `comptime_tree.nika`) and where the crossing rule lives (`dump.nika` `NO_FORM`, Part II 10.2 around line 153).
   - That ADR-384's index row was already fixed on develop.
3. **Checks:**
   - Useful: `check-adr-refs`, fetching `origin/develop` (it settled j3's `Found:` at once), re-running j4's probe on today's binary, the narrowed probes for #1086 and #1087, and the `where.nika` lookup.
   - Wasted: `pre-push-check.sh` printed nothing visible with my `tail`, so I couldn't read its result. The first two `comptime` probes failed on my own syntax (`Contact::fields` used directly, `ref String`), not on the gap I was looking for. Two large `grep`s spilled into persisted output.
4. **The change that would have saved the most:** in `docs/builder-facts.md`, or in the lane's `architect-task.md`, keep a short "reflection and comptime probe skeleton": a working `show[T: Struct]` over `T::fields`, plus an attribute declaration and its use. Every probe of a gap would then start from code that compiles, and roughly six turns would not have been lost to my own syntax errors.
