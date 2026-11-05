1. **What cost the most:** writing the test expectations and filtering test output.
   - `nikaia run` refuses a file inside `tests/compiler`, so I had to copy each test to the scratchpad, run it there and fix the path in the `.stderr` by hand. `nikaia test --bless-stderr` would have done this in one step.
   - My grep for `^test result` also matched test names starting `test result_source::`, so I re-ran the same `it` test command three times.
   - A probe with the wrong list syntax (`[String]`, then `List[String]`; it is `Vec[String]`) cost two more turns.
2. **What the brief could have handed me:**
   - That Nikaia has no `Clone` trait and that bounds aren't available yet (NK1135, NK1126). Its probe with `T: Clone` could never pass, and I found that only by running it.
   - That the fault is in `keeps.nika` `keep_receiver`, in the fallback that counts a receiver as moved when calls don't resolve, not in `text_tiers`. The issue pointed at `text_tiers` and the D12 hand-back.
   - The rule that a file needs one `keep` to raise NK2302 was in builder-facts, which I grepped.
3. **Checks I ran:**
   - One build cycle (`lower-std`, build, `lower-std` fixpoint).
   - `nikaia test --both-settings` with six filters (clone, keep, hand back, is_a_view, still_keeps, generic).
   - The `it` tests for keeps, views_kept, tethers and result_source (133 passed).
   - fmt, clippy, pre-push.
   - **Wasted:** the two repeated `it` runs caused by my grep. The `still_keeps`, `generic` and `hand_back` filters mostly overlapped the `keep` and `is_a_view` runs.
4. **The change that would have saved the most:** builder.md should say how to produce an expected `.stderr`/`.stdout` for a new `tests/compiler` program (`--bless-stderr --filter NAME`). It should also warn that `test result_source::` matches a grep for `^test result`, and give the summary pattern `^test result:`. Separately, a brief should check its own probe against the old binary for refusals that aren't about the defect. The NK1135 for `Clone` would have shown up at once.
