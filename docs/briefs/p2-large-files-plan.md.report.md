Result: done
Files: docs/selfhosting-plan.md
Commit: docs: selfhosting-plan, slices of sysroot.rs, modules.rs, emit/mod.rs (#125)
Body: emit/mod.rs is mostly what stays Rust (with_contracts runs the Rust analyses; the rest are entry points, Build, the Emitter crossing and one-line adapters): only SourceMap and the EmitState table expressions (~160 lines, E13-1) are pure, below a slice, so contracts/mod.rs is planned instead (C1), as the brief directs. Rows: M1 (~380, draft 71%), S2 (~445, 90%), M2 (~205, 36%, goes with M1), S1 (~405, 44%), C1 (~465, 45%), E13-1 (~160, not briefed). S1 and C1 wait for converter fixes (.get(range) on String, else if let, or-patterns with Some in struct patterns, .entry().or_default()). The converter marks Ledger parameters as ADAPTER although Ledger is ty.nika's. First two to brief: M1, then S2.
Fixes: none
Part: #125
Pushed: 6dc116dbdd732fd82e7ab030ee83124e75ecac12
Tests: none, docs only
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: none
