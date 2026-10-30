Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/match_arms_owned_text_result.nika tests/compiler/tests/match_arms_owned_text_result.stdout
Commit: text tiers: to_string() of a number, bool or char is new text, also of a part a pattern binds
Body: The cause was in text_tiers.nika, not views.nika: method_kinds read `x.to_string()` of any receiver with kinds as `viewed(received)`, and a non-text name has owned kinds by default, so `i.to_string()` of an `i64` parameter or of the `i` of `BV::Int(i)` made the function's result a view (`ref String`); NK2302 in views.nika (and NK1104 without the match) followed from that tier. Now a receiver declared as a number, bool or char hands back owned text (ADR-282 D8: the text form of a value), and bind_a_payload binds a scalar payload with its declared type so a match arm's `i` is known as one. Text receivers keep D8's rule (a view stays a view; to_string_of_a_view_fills_a_string still holds). Rows: owned `got` with `t.clone()`, with `t` moved out, `ref got`, `let r = match ...; return r`, and a plain `fn(i: i64) -> String { return i.to_string() }` (also refused before).
Fixes: #998
Tests: match_arms_owned_text_result new (its rows were refused NK2302/NK1104 by the unfixed binary as probes r1, r2, r4, r8; not rerun as a test file there); tests/compiler --both-settings 356 ok; it text_tiers:: keeps:: result_source:: stored_views:: tethers:: text_views:: views_kept:: 255 ok; clippy clean; lower-std fixpoint
Gate: green
Deviation: none
Found: `let s = f"a{n}"; return s.to_string()` in a `-> String` fn is refused NK1104 (text tiers read to_string of an owned local String as a view, the checker as String); also `let s = match got { BV::Int(i) => i, else => 0 }; return s.to_string()` (an untyped let of a number); probe in Body's rows, not fixed here
Needs: none
Changelog: `.to_string()` of a number, `bool` or `char`, also one a `match` arm binds, now hands back text of its own instead of a view that NK2302 or NK1104 refused.
Owner: none
Disk freed: 0 GB
Rs2nika: none
Pushed: rejected
