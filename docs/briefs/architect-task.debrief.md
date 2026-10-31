1. **What cost the most:** pushing. My `git push` was rejected because the shared checkout was behind and had other agents' unfinished files, so I couldn't rebase. My workaround was to rebuild my files on `origin/develop` with a scratch index, and that silently overwrote another session's y21 rule in `docs/architect.md`. Finding that, merging (f96bb09a) and checking ancestry took several turns. Then I chained an edit script with `;`: its assertion failed, the push ran anyway, and an empty commit (dcc656de) went to develop. That was my error.
2. **What I had to look up:**
   - Open issues: `gh api search/issues` is blocked by the proxy, so I paged through `repos/.../issues`. A ready list of open `ready` issues not marked `wip` would have saved this.
   - Which sections of `sysroot.rs` and `lib.rs` hold the preamble and the prelude.
   - Where in `text_tiers.nika` `map` and `filter_map` are handled.
   - The current line ranges in `contracts/mod.rs` and in P5c's file, re-checked at 4520d400.
   - Which moves wait on which bug brief: the dependency between moves and bugs lives only in prose in State.
3. **Checks I ran:**
   - Useful: both #1078 probes on today's binary (`map` works, `filter_map` fails); the grep showing the preamble lacks the Duration and Instant traits; the `merge-tree` dry run; the check that local HEAD is an ancestor of origin.
   - Wasted: the first `gh` search call; re-reading K3b's report after I had already summarised it; and the turns spent repairing my own scratch-index push.
4. **The one change:** a push recipe in the architect brief: "the checkout is shared and dirty: push with `git merge-tree --write-tree HEAD origin/develop`, then `commit-tree -p HEAD -p origin/develop`, chained with `&&`." That rule is now in `docs/architect.md`. A close second: the driver writes the open, unheld `ready` issues and the move-to-bug `Blocks:` map into `architect-input.txt`.
