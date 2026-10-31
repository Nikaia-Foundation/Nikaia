Result: done
Files: crates/nikaia-std/src/tools/emit_pattern.nika crates/nikaia-std/src/tools/lowered/emit_pattern.rs tests/compiler/tests/grammar_a_builtin_takes_a_sequence.nika tests/compiler/tests/grammar_a_builtin_takes_a_sequence.stdout
Commit: grammar: a built-in's one pattern may be a sequence or a choice (#1049)
Body: The run's second parser is the backend's own (winnow-grammar-model parser.rs, behind the `grammar!` macro the emitter writes): `peek(..)` and `not(..)` read one element, `text`/`bytes`/`dec`/`intern` a comma list, so `peek("-" "a")`, `not("+" "a")` and `text("a" | "b")` were refused at the run. `emit_pattern_ref` now writes a lone sequence or choice argument as the group it means; `intern(p)` of one is `intern(text((p)))`, as Part II 10 says it interns p's text; `until(..)` is left as written, the backend reads its alternatives as the terminators it scans for. Probed on the unfixed binary: peek, not, intern and a choice in peek or text failed; until, bytes, span, count, dec, text of a sequence ran.
Fixes: #1049
Tests: grammar_a_builtin_takes_a_sequence (its programs failed on the unfixed binary: peek/not "Unexpected token, expected `)`", intern "takes exactly one pattern"); packages/yaml 14/14
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A built-in that takes one pattern (`peek`, `not`, `intern`, `text`, `recover`) takes a sequence or a choice there, as Part II 10's element table says; they passed `nikaia check` and stopped at the run.
Owner: none
Rs2nika: none
