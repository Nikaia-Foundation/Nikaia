1. **Most turns:** working out how the bounds walk and the prover connect. That meant reading `loop_record_of`, `loop_candidates` and `bounding_loop_stmt`, and seeing that the closed forms are stated over ghost names `x@S`. So the walk needs `x == x@S` at entry and the entry facts restated over the ghosts (`s@ == 0`), or a candidate like `s == i` never proves the goal. The step-1 relative goal also had to be recorded as a disjunction, or `bounding_could_pay` never asks again for `f`'s `j += 1`. Both only showed up while designing step 2.
2. **Looked up that the brief could have given:**
   - The walk keeps no types for names (`BoundsFacts.ints` is a set). Only parameters, with `shape.twice` excluding rebound ones, give a sound type for `y`.
   - Nikaia has no `i8`, and `i64::from` needs `use std::i64`. My first soundness probes failed to type-check because of both.
   - `loop_candidates` was private, and `record.problem.fixed` is the entry facts restated over the ghosts.
3. **Checks run:**
   - Run once per step: the cycle script, the probes under `off`/`on`, clippy, and the ten bounds/prove `it` modules.
   - The `tests/compiler --both-settings` run: about 10 minutes, and wasted. It only showed `drive_actions` failing on the driver's live `target/suite.lock`; the `prove_*` programs could have run alone.
   - The figures run: it had no same-tree baseline, so the brief's numbers weren't comparable.
4. **One change:** the brief should list the integer types Nikaia has and how to write a widening conversion. It should also give a baseline figure per commit (or tell the builder to measure with the pre-change binary first). And it should say "the walk has no name types; use parameters". Those three would have saved most of the probe rework and made step 3's figure meaningful.
