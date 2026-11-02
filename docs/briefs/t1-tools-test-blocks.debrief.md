1. **Time and turns.**
   - The `Walking` struct would not build in a test. I tried five variants, each a `lower-std` round of up to several minutes, before probing the rule once in a scratch file. That probe showed a struct with `ref` fields cannot be built from a local or a `ref` parameter, so the send.rs tests were blocked.
   - The bootstrap loop cost more. The old binary refused `Item::Test`. The index `lowered.rs` held another agent's stale `describe` line, which stopped the build. `text_tiers.nika` was being edited by another agent while I worked. I had to hide the test, lower, build, restore and lower again.
   - One `check` call was left blocked on stdin because of a stray `cat >`.

2. **What the brief could have handed me.**
   - `Parsed` is not `Clone`, so the file has to be re-parsed with `refine_with`.
   - The tools module needs the `ShownBy*` imports before a lowered `assert` compiles.
   - The seam: `lower_one` plus the `with_tests_as_functions` pattern in `modules.rs`.
   - The bootstrap order for a new lowering feature.
   - Ref-field structs cannot be built in tests. I had already hit the same NK1106/NK2302 refusals when I wrote my first threads test, so I should have checked the first test's shape before step 1.

3. **Checks.**
   - Useful: the flip test, `cargo test threads_lowered`, clippy and `pre-push`.
   - Wasted: the four failed `lower-std` rounds on `Walking` variants, and a `cargo check --release` that told me little.

4. **One change.** The brief should have told the builder to probe the first test's shape (build `Walking` from locals) in a scratch file before starting step 1. A line in builder.md would also help: when the shared tree holds another agent's broken `.nika` or `lowered.rs`, say up front how to bootstrap around it. Otherwise the builder rediscovers the same constraints mid-run.
