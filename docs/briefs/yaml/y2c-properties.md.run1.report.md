Result: blocked
Files: none
Commit: none
Fixes: none
Pushed: none
Tests: pre-flight #1026 probe prints 12 34 12 34; the draft from wip/y2-properties applied cleanly onto 9d92e474 and `nikaia check` refuses it at is_float (NK1185, defect 1 under Found); a scratch copy with that call neutralised checks, then all 10 package tests fail in rustc on defect 2; packages/yaml not changed, so the package tests were not run on develop's code
Gate: green
Deviation: step 1 (properties and aliases) not built: blocked on the two compiler defects under Found; the draft is still on wip/y2-properties (ea068fa4), unchanged
Found: (1) a call inside a grammar guard does not count as a call site for text tiers: `entry rule top -> String = s:text(Word) if(fits(s), "ok") eof -> { s }`, `fn fits(s: String) -> bool sync { return ok(s) }`, `fn other(s: String) -> bool sync { return ok(s) }`, `fn ok(s: String) -> bool sync { let body = s.strip_prefix("-") ?? s; return body.len() > 1 }`, and main calls `other(r)` on the parse result: NK1185 at the `??` in ok (left `ref String`, fallback `String`). Moving the fits call into the action (`-> { if fits(s) { s } else { s } }`) checks and runs, and so does an unused `fits` with no `other`. (2) a guard cannot read a text label that has a binding inside: `rule Ch -> i64 = x:any if(x >= '0' && x <= '9') -> { 1 }`, `entry rule top -> String = s:text(d:Ch+) if(s.len() > 1, "two digits") eof -> { s }`: rustc says `Cannot find value s` (4:43). The lowering binds `s` (`grammar::covered(input, __nikaia_s_from..)`) only at the top of the action, so a guard before the action does not see it (#1013 lowering). The draft's Double needs it: `s:text(parts:DoublePart*) ... if(fits(k, s), ..)`. Probes: scratchpad y2c/p4 and y2c/p6
Needs: compiler fixes of both defects under Found (each a `ready` issue, text_tiers.nika for 1, the grammar emitter for 2), then this brief again from wip/y2-properties
Changelog: none
Owner: none
Disk freed: 0.0 GB (target/nikaia-project-tests 5.9G, 11G free on /; no new cache entry over 100 MB)
Rs2nika: none
