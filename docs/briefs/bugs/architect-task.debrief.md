1. **What cost the most.** Writing the #1002 problem took about 6 calls. The issue's version was a bare `entry rule` outside any `grammar` block, which fails to parse. My first fix tripped NK1216 because I'd added `?? "none"`. Before that, I grepped for "Undefined rule" three times and never found where the message comes from; the code probably builds it from pieces. A smaller cost: shell functions don't carry over between Bash calls, so my `mk` helper failed once and I had to rerun it.
2. **What the brief could have handed me.**
   - The CI state: just "RED" with no run id or failing job, so I had to look up runs and jobs myself to see it was an arm64 flake that the next run had already cleared.
   - How a grammar test file is laid out (`grammar X { rule WS = "" -> { } ... }`, `fn main() throws`). I had to copy it from an existing test.
   - That the last run's patch sat in another session's `/tmp` scratchpad. I only noticed by chance that it might vanish, and copied it into the repo.
   - Commit history for #943/#942, which I looked up with `git log`.
3. **Checks.**
   - Useful: the CI run and job lookups, the open-issue list, `git log --grep` per issue, and running each problem on `target/fast/nikaia`. That last one turned up the nullable `ref Vec[T]?` case, which is now in b4.
   - Wasted: the three "Undefined rule" greps, one spec grep whose output was 55 KB, and reading the job log for errors (it printed nothing; the step list answered it).
   - Not done: I didn't open the `docs/architect.md` charter's *Briefs and reports* section. I built the brief format from a previous brief plus `common.part`, which may have missed a rule only the charter states.
4. **One change.** `architect-input.txt` should carry the failing run id, its job names, and whether a later run on `develop` already passed. And any brief that ends `partial` should keep its patch inside the repo (`docs/briefs/bugs/<brief>.patch`), not in `/tmp`. Both are one-line changes to `scripts/drive.nika` and `common.part`.
