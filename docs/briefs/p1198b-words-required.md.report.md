Result: done
Files: crates/nikaia-std/src/tools/check_core_pilot.nika crates/nikaia-std/src/tools/lowered/check_core_pilot.rs docs/specification/30-nikaia-tooling.md tests/compiler/tests/grammar_words_required.nika tests/compiler/tests/grammar_words_required.stderr
Commit: An entry whose result can hold a `Symbol` is refused without `words:` (ADR-388 D2, NK1330)
Body: A call of a grammar entry that interns (it reaches `ident` or `intern(p)`, step 1's `check_grammar_entry_interns`), handed no `words:` and declaring no option `words` of its own, is refused with NK1330 at the call where its result type holds a `winnow_grammar::Symbol`: `check_holds_a_symbol` walks the type's arguments, a tuple's parts, `?`, and a struct's fields, an enum's payloads (positional and named variants) of this file (`structs`, `enum_payloads`) and of the package's other files (`parts_beside`), each type once. The help writes the table and `Grammar::rule(input; words: table)`. An entry that interns only on the way keeps a table of its own without `words:`; the passing half is `grammar_words_required_kept` of the commit before (ba6b114c, which also lets an owned table be lent to `words:`). The NK1330 entry goes into the Part III catalogue before NK1331. For the supervisor: the ADR index row of ADR-388 (`docs/specification/adr/README.md`, `no (#1198)`) can say built.
Fixes: #1198
Tests: grammar_words_required new (.stderr, six NK1330: element, field, variant, a variant's field, `?`, tuple; on the binary before this change the same calls passed the check, no NK1330); grammar_words_required_kept and grammar_words_table_shared ok; tests/compiler --both-settings: 588 passed, 0 failed (the new ones among them); it on errors:: and grammar: 133 passed; clippy clean; lower-std fixpoint holds; pre-push green
Gate: green
Deviation: none
Found: a grammar rule declared `-> winnow_grammar::Symbol?` whose action hands back an `ident` binding (`n:ident eof -> { n }`) reaches rustc unwrapped (`expected Option<Symbol>, found Symbol`); `-> i64?` with a `dec` binding and `-> Named?` with a struct literal are wrapped in Some. Probe: `grammar G { rule WS = "" -> { } entry rule m -> winnow_grammar::Symbol? = n:ident eof -> { n } }` called with `words:`. Suspect: the binding type of `ident` is unknown to the action's wrap (check_grammar / emit of the action's result)
Needs: none
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local dfb6502e34fb7980979308f382a8b0204cda9f83
