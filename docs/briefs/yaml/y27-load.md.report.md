Result: blocked
Files: none
Commit: none
Pushed: none (blocked; the work is on local branch wip/y27-load eef3a2e7, three files: main.nika load + helpers, tests/load.nika, tests/load.stdout)
Tests: wip/y27-load as written fails to build (rustc E0507 in Loader::expansion, shown as NK1127 on every `limits = default_limits()`). With the one line spelled `let held = a.node?.clone(); match held` in a scratch copy of the package, tests/load.nika passes and all 20 output lines were checked by hand (copies, per-document anchors, unknown and recursive alias, a 9-level billion laughs stopped at 5:8 by the limit of 11000, 5 levels at 10560 and built with alias_nodes 100000, an ordinary anchor refused with alias_nodes 0, duplicates a/"a", 16/0x10, [1, 2], alias key, 1.0/1.00; 1/"1" and .nan accepted; `<<` kept). The full package run was not done: nothing was committed
Gate: red
Deviation: none
Found: lowering defect. `match a.node { null => throw .., x => x }` where `a` is bound by `match m.get(k) { null => throw .., x => x }` (a ref into a HashMap) is lowered as `match a.node { Some(x) => x }` without `&`, and rustc refuses it ("cannot move out of `a.node` ... behind a shared reference"). The same match on a `ref A` parameter lowers correctly. Probe (20 lines, `nikaia run`): `struct N { v: Vec[i64] }` `struct A { node: N? }` `fn get(m: ref collections::HashMap[String, A], k: String) -> N sync throws { let a = match m.get(k) { null => throw E{..}  x => x }; let node = match a.node { null => throw E{..}  x => x }; return node.clone() }`, where E is any error struct; lowered: `let node = match a.node { None => .., Some(x) => x, };`
Needs: lowering fix (bug lane): the match scrutinee needs `&` when the place is reached through a local bound to a reference. Then rerun this brief from wip/y27-load: build, `nikaia test --project packages/yaml` (load.stdout already blessed and checked), commit main.nika + tests/load.*. A note for that run: `nikaia test --bless` writes a `.stderr` for every test when the package fails to build (I deleted the 18 it made)
Changelog: none
Owner: none
Rs2nika: none
Disk freed: 0 GB (scratch copy only); target/nikaia-project-tests 3.7G, / 9.3G free; new cache entry over 100 MB: run-bundle/3f8b93b88f7f213d 364 MB
