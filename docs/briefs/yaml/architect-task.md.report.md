Result: done
Batch: y29-1089-match-on-a-get-bound-local.md y27-load.md
Owner: none
Found: #1089 filed (ready, wip; yaml lane): a local bound by `m.get(k) ?? <jump>` or `match m.get(k) { null => <jump>  x => x }` is not lent, so a match or `??` on its non-copy optional field lowers without `&` (rustc E0507); rows a-h probed on target/fast/nikaia at 6eab43c4; kin to #1057 (b13, a refusal), not the same fix.
Decided: nothing new for the language; y28 (91b72357, Fixes #1085) changed `crates/`, Changelog line in its report for the supervisor.
Process: builder-facts *Traps already met* (package commit): no `--bless` before `nikaia check` is clean; NK1127 on a default means rustc refused the lowered file (y27's debrief).
Next after this batch: merge on request (D6, serde_yaml 0.9 `apply_merge`), then tag resolution for events.
Pushed: rejected (local develop holds the batch's unpushed commits; the driver rebases and pushes)
