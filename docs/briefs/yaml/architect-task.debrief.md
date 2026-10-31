1. **What cost the most:** finding where the #1027 defect actually comes from. I read `check_grammar.nika` and `emit_pattern.nika` first, assuming the "Cannot find value" error came from Nikaia's checker. Only after running `nikaia check` (which passed) and `nikaia lower` did I see it came from rustc, through the `winnow-grammar` crate in `~/.cargo/git/checkouts`. Finding what the spec says about labels on groups also took about four greps that found nothing before I reached the 10.8 table.

2. **What the brief could have handed me:**
   - The grammar vocabulary table lives in Part II 10.8 (around line 716 of `20-nikaia-advance.md`).
   - The grammar backend is the pinned `winnow-grammar` crate, and its source is in the cargo checkout.
   - `nikaia lower FILE` writes the emitted `.rs`, which shows what rustc is refusing.
   - The `yaml` lane has no line in `docs/architect.md` *State*; its state lives in #928.

3. **Checks:**
   - Useful:
     - the probe matrix on the current binary;
     - the same probe with the brackets removed, which gave the expected output;
     - `check` against `run`, which pinned the error to rustc;
     - the CI runs query, which showed CI was not red;
     - grepping the draft, which showed it never labels a group.
   - Wasted:
     - grepping for `Cannot find value`, which matched nothing;
     - grepping `--help` for an emit flag;
     - reading `check_grammar`'s binding code;
     - the first `n:(P | Q)` probe, where my `sed` dropped a brace;
     - a `gh api ... -c` call (gh has no `-c`);
     - `git pull --rebase`, which refused to run because of another session's uncommitted files.

4. **The change that would save the most:** the task brief should say: "When a rustc error ('Cannot find value', a type mismatch) comes from a grammar, run `nikaia check` and `nikaia lower` first. The backend is `winnow-grammar` (pinned rev, cargo checkout), so the fix belongs in `emit_*.nika`." It should also say: "When other sessions' uncommitted files block `git pull --rebase`, fetch and push fast-forward instead."
