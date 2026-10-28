1. **Most turns:** working out where each move starts and ends in `project.rs`. Finding where the doc comments begin and the methods inside `impl` blocks end took about 8 small `sed`/`awk` calls. Working out the order of P4b and P1 so neither shifts the other's lines took a few more. Checking the standard library entries one at a time (`chain`, `is_ascii_*`, `split_once`) cost 3 more calls.
2. **What I looked up that could have been handed to me:**
   - The current line ranges, with doc comments included, for each function in a plan row. The plan's numbers were from before P4a.
   - Which standard library calls exist for the Rust calls a slice uses.
   - Whether #557 steps 1-2 were on develop (R0's trigger).
   - How the driver handles a draft under 60 % now (3fff59c had just changed it).
   - The bootstrap-week day row for lane 1.
3. **Checks:**
   - Useful: `git log`/`status`, the P4a report and debrief, the grep for callers, the standard library contract greps, and the issue search for a duplicate before filing #924.
   - Wasted: the grep of Part III for `Seq` (54 KB of output, unused), and some re-reads of line windows I could have printed in one call.
4. **One change:** a script, for example `nikaia run scripts/where.nika -- --cut FILE FN...`, that prints each function's full range `A-B` (doc comment to closing brace) as ready `Cut:` lines, plus whether each Rust standard library call it uses has a Nikaia counterpart. I would then paste its output into the brief instead of hand-computing ranges. That would have saved about half of this run's tool calls, and it would also catch line shifts when two briefs from the same file run one after the other.
