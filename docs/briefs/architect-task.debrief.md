1. **Most cost.**
   - Reading `docs/architect.md` whole took two reads and about 32k tokens. Most of it is the *State* diary (70+ entries), and only the newest entry mattered.
   - My first `git add` named a file I had already renamed (the old b1007 report path). The whole add failed, so the first commit held only the rename and I had to amend it.
   - The push was then rejected because `develop` had moved. I couldn't rebase with other agents' unstaged files in the tree, so my commit 778e5034 is still not on `origin/develop`.

2. **What I had to look up.**
   - Where the checker hands facts to the emitter: `collected_into`, `std_receivers`, `check_core_method.nika:568`, `node_key`/`id_of`.
   - Where `.copied()` is written (`check_reads.nika`, `emit_expr.nika`).
   - The lowered signature for #996.
   - The brief can't hold these, since it was my job to find them. But a one-line "fact map" in `builder-facts.md` (which checker facts reach emit, and how they are keyed) would have saved about 6 greps, for me and for the b1007 builder.

3. **Checks.**
   - Useful: probes of #1007 and #996 on today's binary. Lowering #996 showed the real cause (the `&'static str` tier), and the owned-value variant showed that NK1102 already refuses it correctly.
   - Wasted: `nikaia --help` to find `lower`. The grep for RED handling in the scripts led nowhere, because the stop line was a CUT.

4. **One change.**
   - The architect task should say what to do when the push is rejected and the tree is dirty: leave it to `drive.nika`, or push through a temporary worktree.
   - `architect.md` should move *State* entries older than a day into an archive file, so the charter fits in one read.
