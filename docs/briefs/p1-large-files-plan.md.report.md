Result: done
Files: docs/selfhosting-plan.md
Commit: docs: selfhosting-plan, slices of project.rs and parser/mod.rs (#125)
Body: Adds `## Large files: project.rs and parser/mod.rs`. parser/mod.rs is mostly what stays Rust for now: its grammar! block (3,426 of 4,351 lines) waits for #557 and for a decision on node ids (ADR-339 D3 forbids the Trivia state that 71 actions draw ids from), and R-4 is already built, leaving about 110 pure lines. So comptime_run.rs is planned beside project.rs. Six rows: P2 refusal summary (~420 lines, draft 100%), K1 comptime tree builders (~440, 48%), P1 settings words (~320, 59%), P3 test runner (~720, 86%), P4 packages and members (~660, 88%), K2 (~170, 54%), plus a Stays-Rust list with reasons. First two to brief: P2, then K1.
Fixes: none
Pushed: 9400f22e147a4343c22fcf91ee2c0fb17b1235aa
Tests: none, docs only; converter drafts (rs2nika --profile check --out) over 7 scratch cuts, share = code lines in items without a TODO
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none (answered by the owner 2026-10-28: ADR-383, built by #903)
Disk freed: 0
Rs2nika: none
