Result: blocked
Files: crates/nikaia/src/parser/mod.rs tests/compiler/tests/async_parse_async_is_a_reserved_word.nika tests/compiler/tests/async_parse_async_is_a_reserved_word.stderr tests/compiler/tests/async_parse_overlap_is_a_name.nika tests/compiler/tests/async_parse_overlap_is_a_name.stdout
Commit: parser: `overlap` leaves the reserved words, `async` stays one (ADR-async-5 D5)
Body: Step 1 of #984. `KW_OVERLAP` is out of `RESERVED_B`; `overlap { … }` still parses (keyword first in `primary_expr_bare`) until #991 removes it. `async` was already reserved (`RESERVED_C`, since 0.0.1305), so only its test is new. Rust changed: the grammar rules `RESERVED_B` and the comment above `RESERVED` in `crates/nikaia/src/parser/mod.rs`; no function added. Part I's list of reserved words still names `overlap` (the specification is not this lane's).
Part: #984
Tests: async_parse_async_is_a_reserved_word and async_parse_overlap_is_a_name pass via nikaia test --both-settings on a scratch project; overlap_is_a_name failed on the unfixed binary (`Expected mut here, but found overlap`), async_is_a_reserved_word passes there too (async was already reserved); whole it binary 2887 passed, 0 failed; clippy and cargo fmt clean; no tools .nika changed, so no lower-std
Gate: green
Deviation: step 2 not built
Found: none
Needs: supervisor decision: step 2 needs a check-time refusal for the async forms the later lanes have not built, and no "not built yet" path exists in the checker (no shared code or helper; the only such sentence, `immutable` where an item stands, is a parser `fail`, ADR-389 D4). Either reserve an NK code for "this async form is not built yet", or allow a parser `fail` refusal like `immutable_item` (then no AST node until #986/#988). Also to know: a new `Expr` variant in tools/ast.nika needs an arm in about 47 exhaustive matches over 36 files (counted by `Expr::Overlap` arms), so step 2 is a brief of its own size.
Changelog: none
Owner: none
Disk freed: none (project-tests cache 4.1G, 5.3G free; new over 100 MB: rlib/af187e66 267M, run-bundle/c1ccf85a 288M)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 656a44f6fabd9d5341c4070a82968901c02ed718
