Result: done
Files: crates/nikaia-std/src/tools/parse_findings.nika crates/nikaia-std/src/tools/lowered/parse_findings.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/parser/mod.rs assets/badges/self-hosted.svg
Commit: parser: error conversion and findings from tools/parse_findings.nika (#557 step 4b)
Body: The parser's findings move into a new tools/parse_findings.nika (the subject is the parser's refusals as findings, not parse_errors.nika's words): the 4 GiB reason (fits), char_of/byte_of (as chains over the bytes and chars), finding_of + a_parse_finding (parse_findings_at, building tools/findings.nika's Finding), unclosed_block_comment + a_parse_error (parse_findings_failure, parse_findings_parse_error), a_mut_name_in_a_tuple (NK1307), refuse_parsing's line with its 1-based line and column, and a_types_refusal's finding and line (ParseFindingsRefusal). parser/mod.rs keeps only the adapters: fits (turns the reason into anyhow), failure_of (reads winnow-grammar's ParseError, which Nikaia cannot name), char_of (one call), refuse_parsing/a_types_refusal/refused (traits::from_nikaia and diagnostics::refuse_finding, anyhow stays Rust). Row for scripts/self_hosting.py: "parse_findings.nika": "parser: a parse error, NK1307, a source too long and a grammar-copy refusal as findings, and the line a refusal travels as (#557 step 4b)".
Part: #557
Tests: it `parse errors:: span::` 98 passed; tests/compiler --both-settings 565 passed, 6 failed only with "no such file or directory: target/fast/nikaia" (another agent rebuilt the binary mid-run; trait-column, bounds_expectation, commit_dry_run tests, none parser-related); no .stderr changed; 5 probes (parse error with a non-ASCII name, unclosed comment, NK1307, unclosed paren, grammar file) print byte-identical output on the old and new binary; clippy clean; lower-std fixpoint (second run left parse_findings.rs unchanged); pre-push green
Gate: green
Deviation: step 2 not run: rs2nika is deleted (docs/builder-facts.md, owner 2026-11-01), the move was translated by hand
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 52ea0a2c96f26926e2559f49d5336e1414e0ca77
