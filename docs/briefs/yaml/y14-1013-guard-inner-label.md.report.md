Result: done
Files: crates/nikaia-std/src/tools/check_grammar.nika crates/nikaia-std/src/tools/lowered/check_grammar.rs docs/specification/20-nikaia-advance.md tests/compiler/tests/grammar_a_guard_reads_a_name_bound_inside_text.nika tests/compiler/tests/grammar_a_guard_reads_a_name_bound_inside_text.stdout
Commit: grammar: a guard reads a name bound inside a bound text(..) (#1013 step 4)
Body: grammar_values hands on what the pattern inside a bound s:text(p) binds (Pattern::Bind over the built-in text(..), new helper grammar_a_text_call), so a guard after it in the sequence reads those names as the action does; the emitter needed no change (it already writes p without text(..) and binds its names before the guard, bdbe8a7a). An unbound text(d:P+) keeps its inner names inside for a guard (NK1117), because the emitter does not bind them there; what is below a repeat, a group or a choice stays inside as before.
Fixes: #1013
Pushed: 03d43063b742f9d444c3036affe99ac3ca113fa2
Tests: new grammar_a_guard_reads_a_name_bound_inside_text fails on the unfixed binary (NK1117 at 7:45) and prints 12/error fixed; tests/compiler --both-settings 423 passed (23 grammar_); packages/yaml 11 passed; guard set 24 passed (sequences guard green); clippy clean; pre-push-check green
Gate: green
Deviation: none
Found: an unbound text(d:Ch+) lets the action read d (the checker's bindings_of takes every inner name) but the emitter keeps the backend's text(p), which drops d: rustc "Cannot find value `d`"; probe: grammar G { rule WS = "" -> { } rule Ch -> i64 = x:any if(x >= '0' && x <= '9') -> { 1 } entry rule plain -> String = text(d:Ch+) ";" -> { f"{d.len()}" } } called as G::plain("12;")
Needs: none
Changelog: A guard after a bound `text(..)` in a grammar reads the names bound inside it: `s:text(d:P+) if(d.len() > 1)` (Part II 10, #1013).
Owner: none
Disk freed: 0.1 GB (scratch binary copy; no clone, no target/ of my own; project-tests cache 6.1G, 8.1G free, no new entry over 100 MB)
Rs2nika: none
