Result: done
Files: crates/nikaia-std/src/lib.rs crates/nikaia-std/src/tools/tree_build.nika crates/nikaia-std/src/tools/lowered/tree_build.rs crates/nikaia-std/src/tools/parsed_tree.nika crates/nikaia-std/src/tools/lowered/parsed_tree.rs crates/nikaia-std/src/tools/grammar_types.nika crates/nikaia-std/src/tools/lowered/grammar_types.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/parser/mod.rs assets/badges/self-hosted.svg
Commit: parser: flat_shape, lambda, Parsed's logic and the grammar types' expansion from tools/*.nika (#557 step 4b)
Body: The three steps of the brief in one commit (they share parser/mod.rs and lowered.rs). flat_shape and lambda move into tools/tree_build.nika. Parsed's span_of, kept_function_text and keeping's filter move into tools/parsed_tree.nika (parsed_span_of, parsed_kept_function_text, parsed_kept_items); the Rust methods stay as forwards with their signatures, keeping and growing share a private with_items for the field copies. expand_grammar_types hands the items to grammar_types_whole in tools/grammar_types.nika, which walks every item children first (the order of text_tiers' visit_*_mut, written anew in Nikaia, text_tiers untouched), puts the rule types and records the first refusal with its span; put_rule_types is gone, a_types_refusal stays. A rebound match part is put back by rebuilding the variant (a taking arm). The lowered Cast arm's `Expr::Cast { expr: _, .. }` is clippy's unneeded_wildcard_pattern, allowed on the tools module.
Part: #557
Tests: it filtered parse grammar span 147 passed; tests/compiler --both-settings 571 passed, 7 failed: 6 call target/fast/nikaia by a relative path that is missing from tests/compiler's directory when run by hand, plus agent_script (a /tmp tokens.txt), none touches the parser; cargo test --lib parser:: has 0 tests; errors:: not run: another agent's unfinished describe change stopped the nikaia lib from compiling; clippy clean; lower-std --no-cache fixpoint holds
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
Pushed: local 80bf6cda903e9a33c4d8891b91b05d37baa3ec1e
