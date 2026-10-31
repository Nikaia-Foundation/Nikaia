1. **Most turns or time:** working out step 2. I read about 8 sections of `bounds_walk.nika` to learn that `j += 1` in `f` can't be proved by any invariant. `bounding_fits` caps each sum at `solvable()` (2^62), and the bounds walk only guesses halves of the facts the loop starts with, never the prover's relational invariants. The waits were the two build cycles of about 1 minute each and the gate (whole `tests/compiler`, because `check_facts.nika` changed).
2. **Looked up, could have been handed over:**
   - How the walk puts a loop's record into `scope.loops`: `prover_walk.nika`, the `While` arm (about line 406) and `prover_for` (about line 1113).
   - `ProverScope::renamed` and `rebind`, and how an assignment moves facts into the history.
   - That `--asserts` counts its rows by matching `"proved through"` in `check_facts.nika`, which I found only after the first build.
   - That the bounds walk has its own guess-and-check, apart from the prover's loop records, and the `solvable()` cap.
3. **Checks:**
   - Useful: the start probe on the old binary, the four-function soundness probe, the 9 `it` modules (105 tests, 46 s), and the gate (green).
   - Half wasted: the second build cycle, for the one-line `check_facts.nika` fix; reading the outputs more closely before the first build would have caught it. My first `check` of the new tests ran inside `tests/compiler`, so it checked the whole project and failed on another test.
   - Not run: `lower-std --bounds` for step 3. I left it out because the bounds walk was untouched; it would have been the costly one.
4. **One change:** the architect should check a step's expected result before the brief demands it. Step 2's "`j += 1` in `f` is proved" can't be reached as long as `solvable()` caps the goal and `n` is an unbounded parameter. One `check --bounds` run on `g` (`while i < n { i += 1 }`) would have shown that. A line such as "the bounds walk proves sums only below 2^62 and guesses only from the facts the loop starts with" would have turned step 2 into a decision paper instead of a search.
