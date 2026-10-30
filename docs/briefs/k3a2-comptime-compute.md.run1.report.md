Result: blocked
Files: none
Commit: none
Fixes: none
Pushed: none
Tests: preconditions on origin/develop (k3a1 5bf28f57, #995 0cf0b173, #997 7029fe65, #1005 7bf9dea5); probe of the move's forms with target/fast/nikaia lower: trait method with a fn parameter, the let-bound `known` lambda with `?? ahead.get(name)?.clone()`, set `difference(..).map(clone).collect()`, an enum match with a guard, `map.keys().all(..)`, a generic recursive call all lower; only the set's `iter()` is refused
Gate: none
Deviation: none
Found: `BTreeSet` has no `iter()` (NK1171), and the draft needs it in a chain: `computed` :376 `named.iter().any(fn(name) { left_out.contains(name) || (absent.contains(name) && known(name) == null) })`, the names the value function reads (`names_in_block` fills a set). Probe: `use std::collections` / `fn main() { let mut s: collections::BTreeSet[String] = collections::BTreeSet(); s.insert("a"); println(s.iter().any(fn(n) { n == "a" })) }` gives NK1171 `collections::BTreeSet` has no method called `iter`. std.contracts gives a set only `union`/`intersection`/`difference` as sequences; no ADR refuses `iter` (ADR-collections-1 covers `extend`, ADR-collections-2 covers arrays). Ways around it (a `for` loop, `union(empty).any`, the host handing back a `Vec` instead of the set) are not taken, per the brief
Needs: owner decision (does `collections::BTreeSet` get `iter()`, the sorted lent sequence, like ADR-collections-2 D1 for arrays); then a fresh k3a2 run. The K3a row of docs/selfhosting-plan.md has another session's uncommitted edit (the part 1 entry); I did not touch it
Changelog: none
Owner: Does `collections::BTreeSet` get `iter()` (its elements lent, in order, as `Vec::iter()` gives them), so that `set.iter().any(..)` / `.filter(..)` work?
Disk freed: 0 GB; cache 3.8G, disk 4.1G free; no new cache entry over 100 MB from this run (run-bundle/b2db8b4e959f1cfc 286 MB is older, from b1005)
Rs2nika: none
