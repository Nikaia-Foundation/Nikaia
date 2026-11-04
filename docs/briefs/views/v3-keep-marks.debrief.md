1. **What cost the most:** the whole `it` binary for step 1 took about 30 minutes, and its only two failures were the specification baselines, which were progress and not breakage. After that came working out how to reach the inferred keeps and the keeping statement: I read `keeps.nika`, `contracts/keeps.rs` and `check_against`, and probed the ledger. The run was also interrupted once (`lower-std` exit 144), which cost a resume.
2. **What I had to look up that the brief could have handed me:**
   - The hook point (`check::check_against`) and the API: `keeps_uses_of`, `KeepsIndexes`, `method_index`, `keeps_its`, and the prefix walk in `contracts/keeps.rs::keeping_statement`.
   - That the AST is `tools/ast.nika`, not `ast/mod.rs`.
   - That trait data parameters carry the seed "by value means kept" (#1012 step 3), so per-impl evidence is out of reach.
   - That both code rows already existed in Part III, and that there is no separate codes table in the compiler.
   - How to run one `tests/compiler` program, how to produce a `.stderr` expectation, and that the specification baselines (`EXPECTED.txt`, `COMPILES.txt`) move whenever a parser change makes blocks lower.
   - Gaps in what Nikaia accepts: `Option.clone()` needs `?.clone()`, and there is no slice `to_vec`.
3. **Checks I ran:**
   - Useful: the `lower-std`/build cycle, scratch probes for every form, both binaries on the new tests, `tests/compiler --both-settings` once, `it` filtered (`errors::`, `keep`, `ref`, `self`, `ledger`, `specification::`), `sequences::`, clippy, `pre-push`.
   - Wasted:
     - The step-1 full `it` run could have been `specification::` plus parse-related filters; builder-facts orders the full run for parser changes, but here it only surfaced the baselines.
     - `cargo test --lib check::` matched 0 tests.
     - Running the new tests on the old binary proved little: that binary predates step 1, so every test failed at parsing.
4. **One change that would have saved the most:** a short *Locations* section in the brief, naming `check_against` as the hook, the `keeps_uses_of`/`keeps_its` signatures, the seed limit on trait parameters, and "a parser change that makes specification blocks lower: regenerate `EXPECTED.txt`/`COMPILES.txt`". It would also say to keep the step-1 binary as the "unfixed" binary for step-2 tests.
