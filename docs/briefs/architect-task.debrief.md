1. **What cost the most.** Reading `docs/architect.md`. Its *State* section is about 59 KB of dated entries, and one `sed` printed so much that the output was saved to a file. Running my own probes also took several turns: four probe programs for #938, then the lowered Rust. I also lost a turn on an empty `docs/briefs/RED` file that I first took for the CI status.

2. **What the brief could have handed me.**
   - The CI state of `develop` (I fetched it with `gh api`).
   - That b1029's push had already been rebased and landed.
   - The list of open `ready` bugs, with the yaml lane's issues marked as not mine.
   - Which files b1031 touches, so I could pick a second bug in other files.
   - Where the `for` lowering and `copied_loop_bindings` live (`emit_stmt.nika:960-987`, `check_core_stmt.nika:1502`). I found them with grep instead of `where.nika`.

3. **Checks.**
   - Useful: the #938 probe matrix, including the typed `Vec[i64]` row that showed which forms the bug hits; the brief's full test program (three errors); the lowered Rust; reading `a_copy_by_value`.
   - Wasted: running #1016's probe (I did not pick that bug); a grep for `Ty` variants and integer-literal names that found nothing; reading the `Ty` enum.
   - Not done: I did not confirm the exact type an untyped `[1, 2]` element gets, so the brief marks the cause as unverified.

4. **The change that would save most.** `drive.nika` should write a short status into `architect-input.txt`: the CI run states, whether the last brief's commits are on `origin/develop`, the `ready` issues per lane, and the files the unrun briefs touch. Separately, the dated history in *State* should move out of `architect.md` into `docs/architect-state.md`, keeping only the latest entry inline. Together these would have saved about 6 turns and most of the context I spent.
