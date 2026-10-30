Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/to_string_owned_local.nika tests/compiler/tests/to_string_owned_local.stdout
Commit: text_tiers: .to_string() of an owned or number local stays owned (ADR-282 D8)
Body: A local bound by a `let` to a value made there (not a name, field or element read as it is; new `TierLocal.made`), or by a pattern to a number, whose kinds are owned only and that reads nothing out of another position (no `from`, `through`, `lent`), now keeps owned through `.to_string()`, per ADR-282 D8 (owned text is the same text handed over); a number local is owned in the tiers already (`+`, a scalar payload), so the same rule covers the untyped `let s = match .. { BV::Int(i) => i, .. }` and `let s = n + 1`. The local's kinds were right; `method_kinds` turned them into a view through `viewed()`, so the fix sits at the `to_string` rule, beside b998's scalar-name rule. A view stays a view: a loop variable over a `ref Vec[String]` has owned kinds but is no `let`, so `to_string_of_a_view_fills_a_string` still holds (a first cut keyed on not-a-parameter broke it). `let t = s.to_string()` then moves `s` (NK2105 on a later read), which is D8's "the same text handed over". The brief's row `"x".to_string() + "y"` is refused by NK1301 (to_string on a literal), so the test writes `let a = "x"; let s = a + "y"`.
Fixes: #1005
Tests: new tests/compiler/tests/to_string_owned_local (7 rows: f-string, match number payload twice, `a + "y"`, typed `let s: String`, `n + 1`, `clone`); all 7 rows failed (NK1104/NK2302) on the unfixed binary; tests/compiler --both-settings 357 passed; it guard set + text_tiers:: + views_kept:: 92 passed; clippy clean; pre-push-check clean; lower-std fixpoint holds
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: `.to_string()` of a local holding text of its own or a number is owned text again, so a `-> String` function may return it (ADR-282 D8, #1005).
Owner: none
Disk freed: 0.02 GB (scratch binary copy); cache 3.8G, disk 4.1G free; new cache entry over 100 MB: run-bundle/b2db8b4e959f1cfc 286 MB
Rs2nika: none
Pushed: 7bf9dea5d17e2ec41bd452dcede5179cdec33488
