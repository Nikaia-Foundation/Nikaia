Result: done
Files: crates/nikaia-std/src/tools/check_keep_marks.nika crates/nikaia-std/src/tools/lowered/check_keep_marks.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/check/mod.rs docs/specification/30-nikaia-tooling.md tests/compiler/tests/an_unmarked_kept_parameter_is_refused_in_a_file_that_writes_keep.nika tests/compiler/tests/an_unmarked_kept_parameter_is_refused_in_a_file_that_writes_keep.stderr tests/compiler/tests/a_kept_parameter_marked_keep_is_accepted.nika tests/compiler/tests/a_kept_parameter_marked_keep_is_accepted.stdout tests/compiler/tests/a_keep_never_kept_is_warned.nika tests/compiler/tests/a_keep_never_kept_is_warned.stderr tests/compiler/tests/a_keep_never_kept_is_warned.stdout tests/compiler/tests/keep_self_that_consumes_is_accepted.nika tests/compiler/tests/keep_self_that_consumes_is_accepted.stdout tests/compiler/tests/an_impl_marking_keep_otherwise_than_its_trait_is_refused.nika tests/compiler/tests/an_impl_marking_keep_otherwise_than_its_trait_is_refused.stderr
Commit: v3: check the written `keep` against the inferred keeps
Body: New tools/check_keep_marks.nika compares each parameter's `keep` with the ledger's inferred keeps (keeps.nika, read only): kept and unmarked is NK2302 at the first statement whose prefix keeps it (the walk contracts/keeps.rs does, via keeps_uses_of and keeps_its), only in a file that writes `keep` at least once (transition rule, in the code row; the migration, #1213 step 6, removes it); a `keep` never kept is the warning NK1334 in every file. An impl method marking a parameter or `self` otherwise than its trait's method (trait in this file or the program's others) is NK2302. Not checked: impl methods' bodies against the ledger, since a trait's by-value data parameter is still the seed "by value means kept" (#1012 step 3), and `self`, which the inference does not decide. Rust added: `check::check_keep_marks` (adapter), called from `check::check_against`. Step 1 is 802edb48. self_hosting.py needs a row for check_keep_marks.nika.
Part: #1213
Pushed: local 6f97fbcf8332021a286548da5e6667924371e89a
Tests: 5 new tests fail on the binary before step 1 and pass now; tests/compiler --both-settings 658 passed 0 failed; it filtered errors:: keep ref self ledger specification:: 781 passed; sequences:: 14 passed (unanswered guard green); clippy clean; lower-std fixpoint; committed ledgers unchanged
Gate: green
Deviation: none
Found: lowering: `let help = if c { "a" } else { "b" }` passed to a `String?` parameter lowers as `Some(help)` with `help: &str` (E0308); written here as an f-string around the chosen word
Needs: keeps lane: per-impl body evidence for trait data parameters (#1012 step 3) before an unmarked trait parameter that an impl keeps can be NK2302
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
