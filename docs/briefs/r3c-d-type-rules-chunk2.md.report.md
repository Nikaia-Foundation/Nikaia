Result: done
Files: crates/nikaia-std/src/tools/type_rules.nika crates/nikaia-std/src/tools/lowered/type_rules.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/type_rules.rs crates/nikaia-std/src/lib.rs assets/badges/self-hosted.svg
Commit: type_rules: chunk 2 (expand, literals, given) moves into type_rules.nika (R-3)
Body: wip 6465e74c's two source files applied unchanged (develop had not touched them); lowered file written by lower-std, fixpoint reached. Lowered code trips clippy::extend_with_drain, so it joins the tools module's allow list in lib.rs. self-hosted.svg regenerated from the staged tree.
Part: #125
Pushed: local b271a19043eb68fb68a74fc57469674d2139e55c
Tests: lib type_rules (0 match), it type_rules:: given:: literal_rules:: self_hosting:: 16 ok; 6 tests/compiler *rule* programs ok; clippy clean; pre-push: version red (supervisor's), badge regenerated
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: type_rules.rs: expand, literal check and given-drop now live in type_rules.nika
Owner: none
Disk freed: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
