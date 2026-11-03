Result: partial
Files: see step reports .step1/.step2/.step3.report.md
Commit: dca9b24d (Fixes #1196), e5ebe2ac (Fixes #1197), 9d1461d1 (Part: #557, literal rules STRING STR_CHAR CHAR CHAR_BODY DIGITS DIGIT_GROUP EXPONENT SEGMENT G_BUILTIN G_BUILTIN_WORD F_TEXT F_TEXT_CHAR F_SPEC; step-5 comparison proposal in its message)
Fixes: #1196, #1197
Part: #557
Pushed: local 9d1461d1
Tests: grammar_text_literal_alternative and grammar_span_binds_inner_names new, each failed on the unfixed compiler; parser_grammar_literals new; parser_grammar_words/_trivia re-blessed for literal kinds; it filtered parse/grammar/span/errors:: 185 passed; tests/compiler --both-settings 584 of 585 passed (582 before plus 3 new), agent_script failed on the disk being under 3 GB (agent.nika escalates) and passes singly after the cache cleanup; lower-std fixpoint holds
Gate: green
Deviation: not built: number_lit, negative_number_lit, float_end, FLOAT, the ADR-285 D16 number tests and the refused-number test (need a bound peek); the comment list of step 4 (entry skips WS)
Found: a bound peek(p) passes nikaia check (spec: yields as p) but the winnow-grammar backend refuses it at run ("Peek cannot be bound.", winnow-grammar-model parser.rs:1350); probe: rule NUM -> String = p:peek(text(PRE)) t:text(any{p.len() as i64}) -> { t.clone() }. And the backend's entry point always runs skip_trivia(WS) first (codegen/rule.rs), so a lexical scan through an entry of ParserGrammar loses the comments at the input start: no form for an entry without trivia (kind 2)
Needs: supervisor decision
Changelog: none
Owner: none
Disk freed: 1.6G (target/nikaia-project-tests, under 3 GB free); now 3.3G free
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
