Result: partial
Files: none
Commit: none
Pushed: none
Tests: probed on target/fast/nikaia (winnow-grammar 9e23474): digit, digit+, alpha1, any, char, raw_ident, string, multispace0, multispace1, line_ending, empty, eof all work; only hex_digit (alone and hex_digit+) is refused with "Undefined rule: 'hex_digit'". No test written: nothing to run it against.
Gate: green
Deviation: steps 1 and 2 not built: no file in this repository resolves grammar built-ins. The message comes from winnow-grammar-model/src/validator.rs:249, and its built-in list is the backend's get_builtins (winnow-grammar-macros/src/lib.rs, codegen in codegen/expr.rs:1194ff). That list has digit, digit1, hex_digit0, hex_digit1 but no single-scalar hex_digit. Neither file is in the brief's scope (builder-facts: winnow-grammar is a pinned git dependency, not changed here).
Found: none
Needs: owner decision: either (a) winnow-grammar gets a built-in `hex_digit` (one scalar of 0-9a-fA-F, like `digit`: name in get_builtins, codegen arm beside "digit", first_bytes.rs and frame.rs arms, the "a hex digit" expectation), then the Cargo.toml rev is raised; or (b) the Nikaia emitter rewrites `hex_digit` before handing the grammar to winnow-grammar. (a) matches how `digit` is done. Probe: the issue's program, `nikaia run` -> error: Undefined rule: 'hex_digit'. at 3:35.
Changelog: none
Owner: should `hex_digit` be added to winnow-grammar (and the rev bumped), or rewritten by the Nikaia emitter?
Disk freed: 0 (target/nikaia-project-tests 3.3G, / 7.4G free; no cache entries created)
Rs2nika: none
