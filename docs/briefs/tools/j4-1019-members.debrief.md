1. **What cost the most turns:** three things outside the main work, which was the `members` lowering itself.
   - `match T::kind` refused `Struct(_)` (NK1250). I fixed it in `check_core_pilot`, `emit_core_match` and `grammar_types`.
   - A lowering defect: a `Keep` frame plus `BTreeMap.get` gave rustc error E0277. I avoided it by passing the value directly, and reported it under `Found:`.
   - Two small refusals, each costing an extra cycle: `null` is a reserved field name, and NK2105 (a value used after it was moved).

   Designing the helper rules (`Option<Option<T>>` per member, choosing the value rule by `member_index`, failing from an action with `Cut`) took several reads of the winnow-grammar backend.
2. **What the brief could have handed me:**
   - How a winnow-grammar action fails with a custom error: `ErrMode::Cut` with `Diagnostics::external`.
   - That rules can take value parameters and guards.
   - That `grammar_types` copies rules only for generic grammars, so a non-generic grammar using `members` needs a hook.
   - That `T::kind` arms refused `_` parts.
   - That `null` cannot be an AST field name.
   - That ParseError carries only the rendered text, so the tests must print its first line.
3. **Checks run:**
   - The 5 new tests: they failed on the old binary and passed `--both-settings` on the new one.
   - All 44 `*grammar*` tests and the 7 other `::kind` tests.
   - The `lower-std` fixpoint, `cargo fmt`, clippy and `pre-push-check.sh`.

   **Wasted:**
   - The `parser::` and `grammar::` unit-test runs: there are no unit tests in those modules.
   - One `Struct(_)` probe written with `k[i64?]()`, which the language doesn't accept there, so it needed a second run.

   **Not done as the rules ask:**
   - Clippy ran only on stable 1.97. 1.99.1 is out and was not installed.
   - The report says `Gate: none` because I ran no gate; the driver's suite run decides.
4. **One change that would save the most:** a short *Backend recipes* section in the brief or `builder-facts.md`. It would cover:
   - how a grammar construct lowers to helper rules
   - how an action fails with a custom error
   - value-parameter rules and guards
   - when `grammar_types` copies a rule

   That would have saved most of the reading turns.
