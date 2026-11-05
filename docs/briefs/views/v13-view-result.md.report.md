Result: done
Files: tests/specification/EXPECTED.txt tests/specification/COMPILES.txt
Commit: Spec block #56 lowers again (Fixes #1239)
Body: Part I 4.6's block #56 (`10-nikaia-light.md`), the Box/`get(keep self)`/`hand` program, lowers and compiles again: b28ac489 makes `hand`'s result a view of its argument (ADR-views-3 D12), not a keep. Rust functions changed in the series: refine_with (crates/nikaia/src/text_tiers.rs, calls tier_hand_back); none added.
Fixes: #1239
Pushed: local d15926a3645d858fb193365fe1001cfde5e5afbd
Tests: tests/compiler three new programs (the generic hand-back, an element/field handed back, a moving field taken out still NK2302) pass at --both-settings; the first two fail on the old binary (NK2302/NK1245, NK2105); filters view/keep/generic/hand/number/text 312 ok; it specification:: 5 ok; text_tiers:: unit ok; clippy, pre-push green
Gate: green
Deviation: none
Found: in a keep file, `fn id[T](x: T) -> T { return x.clone() }` gives NK2302 "`id` keeps `x`" (and NK1126, clone on T)
Needs: none
Changelog: A parameter handed back is a view of any type: a function that returns its lent parameter, or an element or field of it, hands back a view of it for every type, not only text, and no longer keeps it (ADR-views-3 D8, D12, #1239).
Owner: none
Disk freed: 0 (cache 2.7G, 3.7G free on /, no new cache entry over 100 MB)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
