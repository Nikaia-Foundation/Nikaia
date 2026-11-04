Result: done
Files: tests/compiler/tests/a_change_after_the_results_last_use_is_accepted.nika tests/compiler/tests/a_change_after_the_results_last_use_is_accepted.stdout tests/compiler/tests/a_change_of_a_source_the_from_names_is_refused.nika tests/compiler/tests/a_change_of_a_source_the_from_names_is_refused.stderr tests/compiler/tests/a_change_while_the_results_view_lives_is_refused.nika tests/compiler/tests/a_change_while_the_results_view_lives_is_refused.stderr tests/compiler/tests/a_reassignment_while_the_results_view_lives_is_accepted.nika tests/compiler/tests/a_reassignment_while_the_results_view_lives_is_accepted.stdout docs/briefs/views/v4-result-from.md.report.md
Commit: Test the caller rule for a result's sources, written and inferred (ADR-views-3 D9)
Body: Step 1 is 5d521f6c (parse and check `from`, NK1335/NK1336, ledger as written; its report docs/briefs/views/v4-result-from.md.step1.report.md lists the Rust added and the self_hosting.py row). Step 2: NK2305 already covers D9 for inferred sources: a change in place before the view's last use and a hand-over to `keep` are refused, a change after the last use and a reassignment are accepted. A written `from` reaches the caller through step 1's ledger change, so `pick -> String from a | b` returning only `a` still keeps `b` unchanged while the result lives. No checker change; tests only. No Rust added in this commit.
Part: #1213
Pushed: local 1aa5c9629c85668a7d6885d2b7ba06ad9d295180
Tests: 4 new tests/compiler programs (the written-`from` one fails to parse on the old compiler; the 3 inferred ones already passed there, coverage only); step 1: it 2886 passed 0 failed, tests/compiler --both-settings 662 passed, clippy clean
Gate: driver
Deviation: no `from` on function types (no parameter names to name); a trait's NK1336 keys on a written `ref` result
Found: NK1261 warns a trait's `-> ref String` redundant although `ref` is today the only mark that a body-less result is a view
Needs: supervisor decision: (1) what `from` names on a function type (positions `from 0 | 1`, or named parameters in function types); (2) after the `ref` migration, what makes a body-less `-> String` a view (`from` alone, or a view by default with NK1336 when `from` is missing)
Changelog: A result may name its sources: `-> String from s`, `-> String from a | b`. The body is checked against it (NK1335), the ledger records it as written, a trait method or foreign function whose result is a view must write it (NK1336), and a caller keeps every named source unchanged while the result is used (NK2305).
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
