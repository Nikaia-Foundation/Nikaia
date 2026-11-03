Result: done
Files: docs/specification/10-nikaia-light.md docs/specification/20-nikaia-advance.md docs/specification/30-nikaia-tooling.md
Commit: specification: keep and from are not reserved, the anchor, NK1333-NK1336 (ADR-views-3 D13)
Body: Part I's reserved-words section says `keep` and `from` are not reserved and mean the mark and the source only in their one position (`from` leaves the ordinary-names paragraph for it). The place a given-up buffer moves to while a view of it lives is the anchor throughout Parts I-III (the Tethered row, the caller's anchor, Part II's tether, Part III's NK2303 and NK2305 entries), defined once in Part I 6.6; "whatever keeps its views" there became "whatever holds its views" so the verb keeps one meaning. Part III's NK1xxx row gains NK1333-NK1336; NK2302 already said a body keeps a parameter not marked `keep` (Part I 6.5), NK1257 and NK1261 stay retired. Step 1 is 90554202 (Array[T] views, D11, D12).
Part: #1213
Pushed: local 762e2e114ce9293a8a53a9c819e289e189843480
Tests: specification:: 5 passed after each step (lowered and floor unchanged, no block added or changed, COMPILES.txt and EXPECTED.txt untouched); pre-push green
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
