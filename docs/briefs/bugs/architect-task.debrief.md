1. **What cost the most.** Tracing #1048 took about six turns. The issue said `AsyncFn`, but only by running a `sync` variant did I see that the `code.is_sync` guard was the cause. Filing #1057 cost one extra turn: I built the JSON in a double-quoted `python3 -c`, so the shell ate the backticks in the title and I had to PATCH it afterwards. The push was rejected, and that only cost time.

2. **What the brief could have handed me.**
   - Whether the CUT commit (b8, 920d8608) was already on `origin/develop`. I had to check `git log` myself.
   - That `common.part` was out of date with `builder.md`'s commit-locally rule. I found it by chance while copying the rules block.
   - That #1056 belongs to the yaml lane. It has no `wip` label; only its body says so.
   - How the architect itself should push in a checkout full of other agents' unstaged files. Rebase, or leave it to the driver?

3. **Checks.**
   - Useful: the reports of b6-b8; the issue list; the probes (b8's Found rows a-e plus the map form, #1048 text, list and `sync` forms); reading `check_core_leaves.nika` and `emit_core_types.nika`; diffing `common.part` against b9's rules block.
   - Wasted: the `nikaia --help` grep looking for a lowering command (I never used it), and grepping for the git history of the `'call` comment.
   - Not run: row d of #1048 (a function kept in a struct field). The brief tells the builder to probe it first.

4. **One change.** In `architect-task.md`, say that the architect commits locally and the driver pushes, as builders do. Also add a small `scripts/issue.nika -- TITLE_FILE BODY_FILE LABELS` that files an issue from files, so nothing passes through shell quoting. Together these would have saved the rejected push, the PATCH and the doubt about rebasing.
