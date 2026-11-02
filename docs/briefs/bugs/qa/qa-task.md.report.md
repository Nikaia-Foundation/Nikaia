Result: done
Cause: 7189b7fc (S1) sysroot_package.nika: sequences::the_corpus_has_no_more_unanswered_method_calls_than_it_had, 16 against 14 - `Item::Test { body, .. } | Item::Bench { body, .. } => body` left `body` untyped (`block.stmts.is_empty()` and `.len()` unanswered); found by bisecting the file.
Fixed: pattern_parts (check_patterns.nika) types an or-pattern's name as the type every alternative gives it, left untyped where they differ (ADR-291 D22); lower-std rerun.
Tests: corpus-count test ok (14, no ceiling change); tests/compiler 549 ok incl. new an_or_pattern_binds_the_type_its_alternatives_agree_on; it sysroot:: 15 ok; cargo fmt; the ceiling test is the guard (the new program passes before the fix too).
Found: none
Filed: #1150 (debug `cargo build -p nikaia` hits the recursion limit in winnow-grammar rt.rs (release/fast build fine, CI is release); not touched, probe: `cargo build -p nikaia`.
Files: crates/nikaia-std/src/tools/check_patterns.nika crates/nikaia-std/src/tools/lowered/check_patterns.rs tests/compiler/tests/an_or_pattern_binds_the_type_its_alternatives_agree_on.nika tests/compiler/tests/an_or_pattern_binds_the_type_its_alternatives_agree_on.stdout
Commit: check: an or-pattern's names are typed where its alternatives agree (fix forward: corpus unanswered calls)
Body: S1's sysroot_package.nika matched `Item::Test { body, .. } | Item::Bench { body, .. }`; an or-pattern's bindings were left untyped, so two method calls on `body` went unanswered and the corpus count rose to 16. The names are now typed where every alternative gives the same type; where they differ nothing changes. Count back at 14.
Fixes: none
Changelog: check: a name an or-pattern binds is typed where every alternative gives it the same type
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 0647154f74622e093bdf08c01f5462572b2f3212
