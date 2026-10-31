1. **What cost the most.** Before the summary, it was reading enough to design the step. That meant `grammar_types.nika`, j1's parser hook, the AST shapes, the kind names, how findings are built and the Rust walker. It filled one context. After that, the pre-existing defect cost about 8 turns: an action's `T` value was not wrapped into its rule's `T?`. Finding it and fixing it in `check_function_action` took that long.
2. **What the brief could have handed me:**
   - The kind names of Part II 10.3. The sketch's `Int` does not exist; it is `I64`.
   - That the catch-all arm is `else`, not `_`.
   - That `check_reflect_arm` already reads an arm.
   - Where NK1164's wording lives.
   - j1's `Finding`/`refuse_finding` pattern in `expand_grammar_types`.
   - `text_tiers::visit_block_mut` as the walker for substituting types in actions.
   - That `f[i64?](x)` does not parse, so a test cannot pass a nullable type at the call.
   - How to run one `tests/compiler` program: `nikaia run` refuses a file inside the project and `nikaia test` has no filter, so I copied files to a scratch project.
3. **Checks run:** the probes on the old binary, `lower-std` twice, `cargo check`, the build, each new test against the unfixed binary, clippy, fmt, `pre-push-check.sh`, `tests/compiler --both-settings`, the `it` binary, and the parser unit tests.
   - Wasted: the parser unit tests. `parser::` matches 0 tests.
   - Partly wasted: running `it` beside `tests/compiler`. Its rebuild of `target/fast/nikaia` made 5 tests fail for no real reason, and I had to re-run them.
4. **One change:** builder.md should say that `tests/compiler` programs spawn `target/fast/nikaia`, so nothing may rebuild it while they run, and it should give the command for running a single test program. A close second: the brief's sketch should use the real kind names, which would have spared me a lookup.
