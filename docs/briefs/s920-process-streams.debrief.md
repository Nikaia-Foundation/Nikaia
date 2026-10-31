1. **What cost the most.** A typo in the `.stdout` I wrote by hand (I dropped `stdout=` from one line). The failure report shows only the actual output, not a diff. So I read how `project_tests.nika` compares `.stderr` before I diffed with `--bless`, which took about 4 turns and 2 extra test runs. Working out how to run one test singly (reading `gate_test_some`, then copying the project by hand) took another 2 turns.
2. **What the brief could have handed me:**
   - That an option after `;` needs no compiler change, only the contract signature and the Rust parameter. I found this from `within`'s commit 2adf2c2c, which touched only `process.rs`, `std.contracts` and the test.
   - That there is already a std enum used as a defaulted option to copy: `par::Order` / `bytes::Scan`, with its `[type.]` block and its derives.
   - That no `tools/*.nika` and no Rust caller uses `process::run` (I grepped to rule out re-lowering).
   - The line where ADR-243's §3 starts, where D10 goes.
3. **Checks run:**
   - Useful: the probe `nikaia run` before the tests; the program test with both settings on a copied project; `it -- process`; the `nikaia-std` `process::` unit tests; clippy 1.97 on the workspace; clippy 1.99.1 on `nikaia-std` only (narrower than the brief's rule, which is a cold build of everything); fmt; `pre-push-check.sh`; `lower-std`.
   - Wasted: reading `project_tests.nika`, and the first failing test run, both caused by the typo.
   - `cargo check -p nikaia-std` before the build was cheap but caught nothing.
4. **One change that would save the most:** add a script `scripts/test_one.nika -- NAME... [--bless]` that does what `gate_test_some` does, and have builder.md say: write a new test's `.stdout` with `--bless` on that copy, then read it, never type it by hand. Second, a brief that adds a std option should name the commit that added the last option (here 2adf2c2c) as the full list of layers to change.
