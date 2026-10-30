1. **What cost the most:** about six `lower-std` and probe rounds to learn three things one at a time:
   - `grammar` is a reserved word.
   - A lambda's parameters cannot carry a type (`fn(why: String)` does not parse).
   - A body-less trait method keeps its `String` and `fn` parameters. `builder-facts.md` already says this for text, and I missed it on the first pass.

   The probe of the moved callback (`Kept`) took four tries. I made my own slips: a Nikaia impl in the probe, `Some(5)`, a `??` with the wrong types.
2. **What the brief could have handed me:**
   - No tools trait so far takes a `fn` parameter, so the sketched host (`sub_program(.., known, typed)`) had never been lowered. A one-line note "trait `fn` params lower as `Kept`; untested when passed twice" would have pointed straight at the defect.
   - The exact names for the consts (prefixed or not).
   - That `Parsed` cannot cross into Nikaia, so the value function's names need a host method.
   - The `let`-lambda rule from b997's commit (a lambda handed to a keeping callee needs its type on the `let`). I found it only by reading that commit.
3. **Checks run:**
   - Preconditions via `git merge-base`: useful.
   - Four `lower-std` runs. One was wasted because the cycle's `grep -E '^error|-->'` misses a parse failure, which prints `Error:` with a capital E. The first run looked clean when it was not.
   - Three probe programs. p3 alone would have been enough had I written it first.
   - The disk check: useful.
   - No build, no tests, no converter `--score`, since nothing compiled.
4. **One change that would save the most:** the brief should come with its host signatures already probed: one small `nikaia run` program per new form (a trait with a `fn` parameter, used twice and handed to itself recursively). This is the architect's rule that "a brief's test rows are probed before it is written", applied to the host trait too. That would have stopped this run in its first turns, before any slice was written. Second, change the cycle's grep in `builder.md` to `'^(error|Error)|-->'`.
