Result: done
Files: crates/nikaia-std/src/tools/project_check.nika crates/nikaia-std/src/tools/lowered/project_check.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/project.rs scripts/self_hosting.py docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: project: the refusal summary of check into Nikaia (P2)
Body: New tools/project_check.nika: project_check_summary (counts only errors, so check passes all findings and drops its filtered Vec), the three severity switches as index loops over mut Vec[CheckFinding], and the three --locked texts; count and plural are module functions. The NK25xx doc comment, which in Rust sat over let_refuted_claims_through, now stands over lint_where_nothing_crosses, the function it describes. project.rs keeps the I/O and refuse(), and calls the lowered functions; 341 Rust lines gone. Self-hosting share 80.0 %.
Part: #125
Pushed: e51e3e2193b4ecfda25902146b2f5ae14a021cb5
Tests: nikaia test --both-settings --project tests/compiler 203 ok; it filtered refus claim locked parallelism sync + guard set 526 ok; nikaia --lib project:: 10 ok; clippy clean; lower-std fixpoint (only project_check.rs and its index lines); self-hosting 80.0 %
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: kept 53 of 123 lines (43 %, below 60 %); costliest edits: closures count/plural turned into project_check_count(findings, ..)/project_check_plural(..) as the brief asked (rewrites 19 f-strings and 23 lets), iter_mut().filter loops to index loops (3x, no Nikaia form), the filtered Vec[ref CheckFinding] dropped for an Error test in the count
