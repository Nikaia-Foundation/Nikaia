Result: done
Files: step 1 39cd60c0 (Parts I-III, adr-372, tests/specification EXPECTED/COMPILES); step 2 8a3be238 (73 files: ADR moves, 5 in-place rewrites, citations in docs/crates/tests, 3 lowered files, scripts/check-adr-refs.py)
Commit: 39cd60c0 spec: write ADR-views-3 into Parts I-III; 8a3be238 Archive ADR-202, 353 and 354 under views-3; rewrite ADR-094, 184, 251, 358 in place
Body: views-3 header citations edited only where the moves broke them; check-adr-refs.py banished() skips docs/briefs (briefs read as written, as in 9b6436ff)
Part: #1213
Pushed: local 8a3be238
Tests: pre-push green; check-adr-refs green; cargo fmt, clippy clean; lower-std fixpoint (3 lowered files: tuple-name renumbering only); spec blocks lowered 66->58 (floor 57), fragment 51->55, refused 68->72
Gate: none
Deviation: COMPILES.txt drops 8 blocks the compiler cannot lower yet (keep/from/mut forms); Part I #85 (D5 let) lowers but rustc refuses it
Found: ref Array[T] refused by D4 but ADR-184 D3/NK1182, the comptime crossing (Part I 2.2, Part II) and Part III windows/chunks still need a view of a run; D2 "returns it by value" vs D8; reserving keep/from open; noun "keep" (ADR-283 D23) collides with the keyword
Needs: supervisor decision: NK codes for written ref, unneeded keep warning, from mismatch, missing from; retire NK1257/NK1261; confirm NK2302 reused for unmarked keep
Changelog: Specification: views-3 written into Parts I-III (keep on kept parameters, mut on changed ones, -> T from a | b, ref only in extern "C"); ADR-202, 353, 354 archived
Owner: none
Disk freed: none; target/nikaia-project-tests 2.7G, / 8.3G free
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
