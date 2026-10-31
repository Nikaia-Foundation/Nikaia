1. **What cost the most:** writing the grammar probes. Five rounds went on my own syntax mistakes, not on findings:
   - character classes inside `text(...)` (`[a-z]` does not parse);
   - `??` with an escaped quote inside an f-string interpolation;
   - a `ref mut` parameter, which is NK1158;
   - a bare variant pattern, which turned out to be a real defect (#1083).

   After that came working out how `name:` could be evaluated while building, a design I then put off to j5.
2. **What the brief could have handed me:**
   - That the reflected field's type is the internal `$Field`, and which files handle it.
   - That `comptime` parameters (ADR-specialization-1 D1) have no tests today, so nothing shows them working yet.
   - That NK codes have no registry: each code is written where it is refused.
   - Where Part III lists NK1310–1312.
   - A grammar probe already written for the lane's current form (a `match` on a kind plus a struct).

   The builder-facts probe to copy exists, but my task brief did not name it.
3. **Checks run:**
   - Probes on `target/fast/nikaia` for `members`, `meta::Field` and attributes. These were useful and went into the briefs.
   - `where.nika` outlines. Useful.
   - The rules block of each brief checked against j2's text.
   - Wasted: grep searches for a registry and for `comptime` parameter tests, which found nothing.
   - Mistake: I ran `pre-push-check.sh` after committing, not before. That is how the ADR-384 failure showed up late, and the push was rejected anyway.
4. **The change that would save the most:** the lane's `architect-task.md` should list the lane's working probe files (j1/j2's tests as templates), name the state of the forms the next step rests on (`$Field`, `comptime` parameters untested), and say to run `pre-push-check.sh` before committing.
