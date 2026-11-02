Result: partial
Files: docs/briefs/m6-send-tests.md.report.md
Commit: briefs: m6-send-tests report
Body: Per-piece commits 7656bd20 (emit fix), cb3c303b (send), 5162a642 (ty), 97a4bcf2 (touch), 8bb24290 (clippy pub helpers ty/touch), 3c7a0e1a (manifest). Tests are Nikaia `test` blocks in threads/ty/touch/manifest.nika (lowered to #[test], run with `cargo test -p nikaia-std --features compiler --lib -- nikaia_tests::`), not tests/compiler programs: they call Ty/Ledger/manifest_parse, which a program cannot reach. send 11/13 moved, ty 14/14, touch 9/10, manifest 12/15. Stay in Rust: send's two std-ledger tests (Ledger::parse is Rust), touch's KINDS test, manifest's flag test, toml::Value dependency pass-through, codegen-units test (blocked, see Found). Self-hosting share 85.5 % (not measured before).
Part: #125
Pushed: local 3c7a0e1a
Tests: nikaia-std nikaia_tests for threads 11, ty 14, touch 9, manifest 12 ok; nikaia --lib contracts::/manifest:: ok; tests/compiler program a_ref_field_lent_a_local_of_its_own_name fails on the old binary (rustc E0308), ok on the new. Clippy not clean-verifiable: other agents' unfinished contracts/mod.rs (unused imports) stopped it; my files showed no warnings after the pub change.
Gate: red clippy (not mine: contracts/mod.rs unused imports Item, TyOps)
Deviation: tests are tools-file `test` blocks, not tests/compiler programs (reason in Body); codegen-units test not moved.
Found: tuple list literal with a nullable slot does not lower: `let rows = [("a", 0), ("b", null)]; for (t, n) in rows { show(n) }` with `fn show(n: i64?)` gives rustc E0308 (no Some() around 0, `.into()` on integer), also with `Vec[(String, i64?)]` annotation. Fix belongs in the emitter/checker widening for tuple elements.
Needs: none
Changelog: emit: a local lent to a ref field under the field's own name keeps its & (it was written as the shorthand and failed in rustc).
Owner: none
Disk freed: none; target/nikaia-project-tests 2.6G? (du: 2582 MB avail on /)
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
