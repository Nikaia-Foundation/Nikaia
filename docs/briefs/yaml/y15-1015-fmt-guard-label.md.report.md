Result: done
Files: crates/nikaia-std/src/tools/fmt.nika crates/nikaia-std/src/tools/lowered/fmt.rs tests/compiler/tests/fmt_keeps_a_grammar_guard_whole.nika tests/compiler/tests/fmt_keeps_a_grammar_guard_whole.stdout assets/badges/self-hosted.svg
Commit: fmt: keep a grammar rule's parentheses whole, a rule's continuation one level deeper
Body: The printer's frame now knows a `grammar { }` body (`FmtFrame.rules`, set by `fmt_opens_grammar` on the line that opens it). On a line in such a body, `reshape` treats every `(` group as sealed (a guard's `if(..)`, `fail(..)`, a group): the grammar parser takes no break and no trailing comma inside them, so a long rule line breaks its action block or stays long. `indent_of` puts a body line that starts no rule (`fmt_starts_rule`: `rule`, `entry`, `pub`, `@frame`) one level deeper than the rules, so a rule broken by hand keeps its continuation, and formatting again changes nothing. Both steps of the brief in one commit. The badge is redrawn because the self-hosting share moved.
Fixes: #1015
Pushed: 2e553fe50db1667bb396de65f5d733b3402a2481
Tests: fmt_keeps_a_grammar_guard_whole (probe 1 + probe 2's hand-broken rule) red on the unfixed binary (fmt: "the formatted file does not parse: Expected `)`"), passes now and is a fixed point; it::fmt:: 18 passed; tests/compiler --both-settings 422 passed; guard set 24 passed (sequences within its ceiling); clippy clean; pre-push-check ok; wip/y2c-properties packages/yaml/src/main.nika formats, passes `nikaia check`, and is a fixed point (fmt re-indents its unformatted action blocks)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: `nikaia fmt` keeps a grammar rule's parentheses (guard `if(..)`, `fail(..)`) on one line and puts a rule's continuation lines one level deeper than the rule
Owner: none
Disk freed: 0 GB (no clone; target/nikaia-project-tests 6.1G, 8.3G free on /)
Rs2nika: none
