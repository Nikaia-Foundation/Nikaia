1. **What cost the most turns:** pushing at the end. Step 8 of the task says to rebase when a push is rejected, but `git pull --rebase` won't run while other agents have unstaged files in the shared checkout, and stashing them is forbidden. I then amended the commit three times to get its own sha into the report, which can't work because each amend changes the sha. After that came finding where the `hex_digit` refusal comes from: about 4 greps before I found it is in the pinned `winnow-grammar` crate. `docs/builder-facts.md` already says this, but I hadn't read it.
2. **What I looked up that the brief could have handed me:**
   - How to see the lowered Rust (`nikaia lower F.nika`); I found it by grepping `docs/builder-facts.md`.
   - That the grammar backend is external and isn't changed here.
   - Which files the other lanes' running briefs touch. I opened b938/b1034a/b1035 to check overlap, and that is how #947 turned out to collide with #938 in `check_core_stmt.nika`.
   - Some `std` and syntax facts. Three of my probes failed on my own mistakes (`Vec[String]()`, `.slice`, `["a".to_string()]` into a literal list).
3. **Checks:**
   - Useful: re-running each issue's probe; the widened probes (head chain for #939, the reduced #1016, the number-list case for #947); checking the commits that mention each issue; the temporary-argument probe, which found #1036.
   - Wasted: the three probes broken by my own syntax errors, and the push retries and amends.
4. **One change that would save the most:** rewrite step 8 for the shared tree. Push fast-forward once; if it is rejected, write `Pushed: rejected` without a sha and stop, and the supervisor rebases. Also have the driver write the files each running brief of the other lanes may touch into `architect-input.txt`, so overlaps like #947 against #938 show up without opening their briefs.
