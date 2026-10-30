Result: done
Files: crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/check_core_method.nika crates/nikaia-std/src/tools/emit_core_calls.nika crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/check_core_method.rs crates/nikaia-std/src/tools/lowered/emit_core_calls.rs crates/nikaia-std/src/tools/lowered/project_check.rs tests/compiler/tests/a_typed_list_summed_needs_no_annotation.nika tests/compiler/tests/a_typed_list_summed_needs_no_annotation.stdout
Commit: check, emit: sum() names its result type (#1007)
Body: A new checker fact, summed (receiver node -> Rust number type), is written by check_core_method_called when a sequence's sum (receiver typed Ty::Seq) hands back a plain number Part I 2.2 offers; emit_core_method_call_body pushes ::<T> after sum from it. The fact is keyed by the receiver, as collected_into is, because on_node at that point is node_key(receiver) on both callers. lower-std also rewrote project_check.rs, which calls sum (now .sum::<i64>()). The test's .stdout is the real run's output (f64 lines print 3.5).
Fixes: #1007
Tests: new a_typed_list_summed_needs_no_annotation failed on the unfixed compiler (rustc "Type annotations needed"), passes now; nikaia test --both-settings --project tests/compiler 366 passed 0 failed (incl. a_number_list_literal_summed_is_typed); lower-std fixpoint clean; clippy clean; pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: sum() of a sequence of typed numbers (v.iter().sum() with v: Vec[i64]) names its result type, so a program rustc refused with "type annotations needed" now runs.
Owner: none
Disk freed: 0 GB (no clone)
Rs2nika: none
Pushed: ad7f118fd8f892614efb15dabbd80a9980d00e03
