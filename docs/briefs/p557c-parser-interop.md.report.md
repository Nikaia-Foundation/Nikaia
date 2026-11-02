Result: noop
Files: none
Commit: none
Body: #557 step 4a answered no: the Rust `grammar!` block cannot call a rule of another `grammar!` block (winnow-grammar 4f8e6691). A rule the block does not define is called as the parser value `&mut path` with `ParseError`; another grammar has no such value. `Leaf::parse_word` is a function that returns the entry parser, so rustc says E0593 "function is expected to take 1 argument, but it takes 0 arguments". `Leaf::parse_word_inner` is private (E0603) and generic over `__WgEr` (E0308 against `ParseError`). `Leaf::word` does not exist (E0425). A hand wrapper `extern rule word_ext` around `Leaf::parse_word().parse_next(i)` compiles but is an entry point, not a sub-rule. `rt::entry` runs `begin_parse` (clears the outer parse's `furthest`, rule stack, recoveries and trivia memo) and runs the fast and then the diagnosing pass again. It skips the callee grammar's WS before and after the rule, so trailing whitespace is eaten without the outer `WS`/`Trivia` recording it. `rt::finish` demands end of input. Probe: `a:word_ext b:word_ext` on "x \n y" fails with "expected end of input", while the same rule written locally gives Ok(("x","y")) with 1 trivia run recorded against 0. The input type (text, `ParseInput<'a,S>`) and the state (a stateless grammar is generic over any `S`) would agree; the call mechanism is what is missing. So per #1188 the port runs side by side and switches in one commit, unless winnow-grammar gets a sub-rule export: a `pub` `parse_X_inner` generic over `__WgEr`, called from another grammar by path (`Other::x`) with the caller's `__WgEr`, no entry, no trailing skip, no eof check. That is a change in the owner's crate.
Fixes: none
Pushed: none
Tests: noop: only a scratch crate outside the repository was probed (/tmp/claude-0/-home-user-Nikaia/b7e2d91b-1a15-5371-a36e-8c201aeab2d2/scratchpad/p557c-parser-interop/probe/src/main.rs, winnow-grammar rev 4f8e6691, Leaf/Outer grammars, three direct path forms plus the wrapper); no compiler build, no repository test needed
Gate: green
Deviation: none
Found: none
Needs: supervisor decision: chunk by chunk needs a winnow-grammar sub-rule export (owner's crate, see Body); otherwise the plan is side by side and one switch commit
Changelog: none
Owner: none
Disk freed: none; the probe's target dir (174 MB, scratchpad/p557c-parser-interop/target) is left because its removal was denied; target/nikaia-project-tests 197M, / 5.2G free
Rs2nika: none
