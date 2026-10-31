1. **Most time:** proving the new test fails on the unfixed compiler. No older binary existed (no `target/release`, no `target/debug`), so I put back HEAD's `fmt.nika` and `fmt.rs` by name, rebuilt, ran the test, restored my files and rebuilt again: two extra builds. The `tests/compiler --both-settings` run was the longest wait, but the gate requires it.
2. **Looked up:**
   - What a grammar body may hold (`grammar_rule`, `frame_attr` in `parser/mod.rs`), needed to tell a rule's first line from its continuation.
   - That a `here` grammar needs a `Bytes` input (NK1187), and how to write one (`Bytes([])`).
   - That `grammar` is a reserved word and cannot name a struct field; this cost one cycle.
   The brief could have given all three in one line each.
3. **Checks:**
   - Useful: the probes on the old binary, the `fmt::` tests, the guard set with clippy alongside, `pre-push-check` (it caught the badge), the full `tests/compiler` run, and the yaml-draft check.
   - Wasted: the first build cycle. `lower-std` refused the reserved field name, but the chained `cargo build` still ran on the old lowered code.
4. **Biggest saving:** builder.md could say "keep a copy of `target/fast/nikaia` before your first build (`cp` to the scratchpad)". The test against the unfixed compiler then takes seconds, not two rebuilds. Second, add to the builder-facts syntax gaps: "`grammar`, `rule`, `entry` are reserved words; never use them as names in `tools/*.nika`."
