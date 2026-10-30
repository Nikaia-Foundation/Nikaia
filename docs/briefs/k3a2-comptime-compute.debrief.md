1. **What cost the most.** Reading the cut and planning the whole `ComptimeComputeHost` (arena indices, `driven`, `comptimes_of`) before checking whether the draft's chains compile. One `lower` of a small probe showed the blocker. Two turns went to my own slip: I gave a lambda a typed parameter, which Nikaia doesn't parse.

2. **What I had to look up that the brief could have given me:**
   - the `BTreeSet` methods in `std.contracts` (no `iter`, only `union`, `intersection` and `difference` as sequences);
   - whether an ADR settles set `iter()` (ADR-collections-1 covers `extend`, ADR-collections-2 covers arrays, and `selfhosting-gaps.md` lists only `extend`);
   - how closure parameters lower in a tools file (`&impl Fn`).

   The brief warned that sets lack `iter`/`filter`, but only for `absent`. It missed that the draft itself has `named.iter().any(..)` on a set at :376.

3. **Checks I ran:**
   - precondition `git log` greps;
   - a first probe `lower` (stopped by my syntax slip, half wasted);
   - a second probe (found NK1171, and showed the other forms lower);
   - a third probe with `iter()` swapped for `union`, which ruled out a second gap;
   - a 3-line `run` probe that confirmed it on a plain set;
   - the disk check.

   I also read the lowered `comptime_sub_program`/`ComptimeLowerHost` signatures before the first probe, which turned out not to be needed.

4. **The change that would have saved the most:** before a brief goes out, the brief writer (or the converter's `--lint`) should run each of the draft's chains against the receiver's methods in `std.contracts` and list any that would get NK1171. Here that catches `BTreeSet.iter()` at :376, so this brief goes to the owner as a decision instead of a builder run. Failing that, builder.md should say: lower the draft's chains in a probe before you design the host.
