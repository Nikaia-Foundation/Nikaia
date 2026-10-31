1. **Most cost:** the streamed fold in `src/rt/stream.rs`. I needed three reads (the `Fail` enum, the item loop, how `chunk`/`step` handle `Stop::Failed`, and `attempt`) to be sure that numbering a Cut at both 515 and 868 never numbers the same error twice. After that came two test-harness slips. My first helper was generic over `Parser<…, ErrMode<ParseError>>`, but the generated parsers' error type is `ParseError` itself, so I switched to a macro like the one `diagnostics.rs` uses. And the brief's "no `item` frame" check for `[{x}]` was too strict: `digit+` inside the element adds its own `item 1`.
2. **Lookups the brief could have handed me:**
   - That the generated parsers return `ParseError` directly, and that tests read `rule_stack` the way `diagnostics.rs` (`p12_error_is_structured`) does.
   - That `repeat_recording` and `repeat_counting` only delegate to the bounded forms.
   - That `attempt` is the diagnosing re-run fed by `Some(*n + 1)`, so a Cut there is numbered once, with no double count.
   - That a `digit+` inside the element adds its own `item 1` below `item`.
   - That clippy on 4f8e6691 is already red.
3. **Checks run:**
   - `cargo test --test cut_test`: twice to fix the test, then once after fixing my wrong expectation.
   - The fails-before runs: the whole test with `src` stashed, then the `ababac` case alone, because the first assertion failing hid the third.
   - Full `cargo test`, `cargo clippy`, `cargo fmt --check`. Nothing ran in the Nikaia checkout: no build, no pre-push check.

   Wasted: one test run caused by the generic-helper compile error, and a second clippy run just to read the lint text, which a first run without the `grep` filter would have shown. Also, `Gate: green` in the report is not backed by a gate run, since the brief forbids one. The field should say `not run` for a backend-only brief.
4. **One change:** a backend brief should say whether clippy and the tests are already green at its base rev, and give the test snippet to copy (`Stateful { state: ParseContext::<()>::default(), input: LocatingSlice::new(src) }`, then `.parse_next(&mut s).unwrap_err().rule_stack`). Its expected stacks should come from a probe run, not be written from memory.

   Also, after `commit.nika` ran, the committed report no longer shows the `Rs2nika: none` line I wrote. The supervisor should check whether `commit.nika` drops the report's last line.
