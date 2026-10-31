1. **Biggest cost:** my first commit. I chained `git add ... && git status; git commit` with a pathspec for a file that no longer existed after my `git mv`. The add failed, but the `;` let the commit run anyway, so db275fc8 holds only the renames and a second commit (f5a6fd29) was needed. Next was working out where fmt lives: I looked in `tools/`, then `crates/nikaia/src/fmt.rs`, before finding `crates/nikaia-std/src/tools/fmt.nika`.
2. **What I looked up that the brief could have handed me:**
   - Where fmt lives and its main functions (`reshape`, `fmt_broken`, `fmt_sealed`).
   - That `fmt::every_nika_file` scans `tests/` as well as `packages/`.
   - That an existing `tests/compiler` fmt test is the model for a new one.
   - That the debrief files are tracked, so renaming them for a re-run needs `git mv`.
   - That the main checkout held run 2's leftover `nikaia.derived` and `nikaia.lock`.
   
   `architect-input.txt` could also name the branch the blocked run left behind (`wip/y2c-properties`).
3. **Checks:**
   - Useful: the two fmt probes with the existing binary, which confirmed both defects and gave the brief exact symptoms. Also `git ls-remote` for the wip branches, the diff stat of the branch against develop, and a grep for the `fmt::every_nika_file` directories.
   - Wasted: the greps for fmt under the wrong paths, and the `git status` before commit that never ran.
4. **The one change:** a line in the architect task's *Lane facts* (and in `docs/architect.md`): "commit with separate calls, `git add <files>` then `git commit` only if the add succeeded; never `;` between them, and check `git show --stat HEAD` before pushing."
