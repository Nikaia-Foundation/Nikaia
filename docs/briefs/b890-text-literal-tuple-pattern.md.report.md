Result: done
Files: crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/emit_core_places.nika crates/nikaia-std/src/tools/lowered/check_core_flow.rs crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/emit_core_places.rs tests/compiler/tests/a_text_literal_in_a_tuple_pattern.nika tests/compiler/tests/a_text_literal_in_a_tuple_pattern.stdout tests/compiler/tests/a_literal_is_the_same_part.nika tests/compiler/tests/a_literal_is_the_same_part.stdout tests/compiler/tests/a_different_literal_is_not_the_same_part.nika tests/compiler/tests/a_different_literal_is_not_the_same_part.stderr
Commit: match: a text literal in a tuple pattern; a literal is the same part
Body: Steps 1 and 3 of #890 in one commit (both in check_core_flow.nika). A `match` over a tuple written in place, `(a, b)`, with a text literal at a position whose part is an owned `String` or `String?` (through `|` and nested tuples) now reads that part as its view: the checker types the position as `str` / `str?` (so names bound there in other arms are views, as with a top-level text match, #704) and keeps it in `text_parts`; the lowering writes the part `&*a` or `a.as_deref()` in the subject tuple, so the literal is a `&str` pattern and `Some("out")` matches an `Option<&str>`. Other positions keep their types. A subject that is a tuple variable, not written in place, is not covered. And ADR-352 D1 as amended: `check_core_flow_same_part` says true for two equal literals (text, integer by value and sign, `bool`, `char`), so after `(false, null)` the arm `(false, w)` binds `w` as the `T`; `(1, null)` before `(2, w)` still leaves `w` a `T?` (NK1125). Tests: a_text_literal_in_a_tuple_pattern (failed before: Mismatched types), a_literal_is_the_same_part (failed before: NK1125), a_different_literal_is_not_the_same_part (NK1125 before and after).
Fixes: #890
Pushed: f6bba7874bac7328547c46c4cc02777796d57be8
Tests: 3 new program tests, the first two failed on the unfixed compiler; nikaia test --both-settings --project tests/compiler 179/180 (agent_script red from 26db5c6, not this change); guard set green; clippy, pre-push-check green
Gate: green
Deviation: none
Found: agent_script test red since 26db5c6 (scripts/agent.nika cost cap 200 to 1000 USD; agent_script.nika still writes 200.2 USD and expects ESCALATE)
Needs: none
Changelog: a text literal at a String or String? position of a tuple pattern is matched against a view of that position (#890); a literal at a tuple position counts as the same part for narrowing (ADR-352 D1)
Owner: none
Disk freed: 0 GB (main checkout, no clone)
