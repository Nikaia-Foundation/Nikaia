1. **What cost the most:** writing the probes for #995. Several of them failed on syntax I had wrong, like `BTreeSet::new`, `.to_string()` on a literal, typed lambda parameters and `.copied()` on `T?`. After those, the next biggest cost was reading `comptime_run.rs` and `comptime_tree.nika` to sketch the K3a part 2 host. A `git log -S` search also timed out and had to finish in the background.
2. **What I had to look up:**
   - Syntax for probes: constructors are `collections::BTreeSet()`, and lambda parameters are written without types.
   - Where `Wall`, `Bounds`, `LedgerOps::absorb`, `run_comptime`, `dumper` and `with_driver` live.
   - That sets have no `iter`/`filter` in `std.contracts`.
   - Why k3a1 left its work uncommitted (the 30bdbc66/9538c0b0 history).

   A cut report that showed `git status` for the cut builder's files, plus a "probe syntax" cheat sheet in `builder-facts.md`, would have covered most of it.
3. **Checks:**
   - Run: the probes through `nikaia`, greps for who calls the constants and adapters, `git status` and the diff stat, and the issue bodies through `gh api`.
   - Partly wasted: the `git log -S` search, since `git log` on the file was enough. The probes ran on a `target/fast/nikaia` built before 0.0.1307, so they may be stale; the briefs tell builders to re-probe on a fresh build.
4. **The change that would have saved the most:** have `batch.nika` refuse a report with no `Pushed:` sha or with a placeholder (`TC`, `DISK`) left in, and say why in the cut line (for example `CUT ...: not committed`). It should also attach the cut builder's `git status --short` to `architect-input.txt`. That would have saved my whole diagnosis. I already added the matching builder-side rule to `common.part`.
