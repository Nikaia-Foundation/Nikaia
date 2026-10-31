Result: blocked
Files: none
Commit: none
Fixes: none
Pushed: none (nothing committed to develop; draft advanced on local branch wip/y21-events 1c1199d6, parent 0433d662)
Tests: pre-flight passed (y24 probe `hello`, y25 (a) `1 hello` and (b)+(c) `2 hello`, y26 `1 hello` on a freshly built target/fast/nikaia); draft applied, nikaia fmt clean on both files; `nikaia check packages/yaml/src/main.nika` leaves one NK2302 (the NK1127 on `default_limits()` was a cascade and went once the others were fixed); package tests not run (still blocked)
Gate: red
Deviation: none
Found: a struct or list holding views of a `ref` parameter, returned through an `if` or `match` expression, is refused NK2302 ("`one` keeps `n` after this call, but `n: ref N` doesn't say whose data it points into"); `let r = [E { name: n.name }]; return r` passes. Probe: `struct N { name: String }`, `struct E { name: String }`, `fn one(n: ref N) -> E sync { return if true { E { name: n.name } } else { E { name: n.name } } }`, main: `let n = N { name: " hello ".trim() }` then `println(one(n).name)`. Wanted `hello`. Shapes refused the same way: `return match n.value { V::One(m) => [E { name: n.name }] else => [] }`, a block arm ending in `out`, and `let r: Vec[E] = match ... ; return r`. Suspected place: views.nika, the result-source walk (`rsrc_*`, near `rsrc_function`), which does not follow the arms of an `if`/`match` expression to the value it hands back. walk in packages/yaml returns `match n.value { ... }` with these arms.
Needs: architect decision (a fix brief for the defect above, then run 4 takes `git diff 6269eca3 1c1199d6 -- packages/yaml/src/main.nika packages/yaml/tests/events.nika | git apply`). What run 3 changed on the draft: `Event.value` is now always a view (`Node.raw`, or an alias's name), and a new `pub text: String?` holds the owned `text(s)` of a `Value::Str` (null for every other event). This removes the two NK1106 errors that came from `value` mixing owned `text()` output with views. event_line writes `text` if it is set, else `value`.
Changelog: none
Owner: none
Disk freed: 0 GB (target/nikaia-project-tests 3.7G, / 11G free; no cache entries created in this run, since no package test ran)
Rs2nika: none
