1. **Most time:** the reproduction itself, about 10 minutes for the whole `it` binary with the dump, then 1 minute for the sweep. After that, finding the two causes took about 8 turns of `git log` work. The brief's three suspects all pointed at eeaefb1, but the failures came from two other things: a hash changed by a later commit (79d6e81), and a `main` check in the sweep that a string literal fooled. One wasted rebuild came from a type I missed on a `let` (E0308).
2. **Looked up, could have been handed over:**
   - The AST path to ask whether a program declares `main` (`unit.parsed.program.items`, `Item::Fn { name }`, `parsed.text`). `grammar_run.rs:304` already does this.
   - The rule that an allow-list entry whose hash no test produces any more also fails the sweep.
   - That commits after the first red push (79d6e81, 7e3f8eb) can add failures of their own, so `git log 590cd7c..HEAD` is the first thing to read.
3. **Checks run:**
   - The full `it` dump and the sweep were needed, since they reproduced the failures.
   - Re-running the sweep over the same dump was needed, and cheap.
   - The guard set, clippy and `pre-push-check` were needed and quick.
   - Reading eeaefb1's `index.rs` and `emit` diffs turned out not to matter (suspect 1 played no part).
   - Nothing was clearly wasted apart from the compile that failed on E0308.
4. **The change that would save the most:** have the brief tell the builder to start from the current tip, not the first red commit. The step would be: reproduce at HEAD, then classify each failing hash by `git log -S` on its source, before testing any suspect. The brief's cause ("eeaefb1, so suspect 1, 2 or 3") was a guess. Saying "suspects are hypotheses; reproduce, then map each hash to a commit" would have pointed straight at the right lines. Also allow updating an allow-list hash when the same program was re-spelled; that is a mechanical case, not a new allow-list entry.
