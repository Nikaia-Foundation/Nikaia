1. **What cost the most:** about a third of the run went on compiler failures that only showed up as rustc errors. The grouped alternative, `hex_digit`, `8232 as scalar` and the temporary `Bytes(..)` each needed its own probe to confirm it was a defect before I could decide what to do. The other big cost was `--bless`. Before I had made one test's `.stdout` by hand, it reported "blessed" or "ok" with empty output, and I could not tell why. I spent several turns suspecting `nikaia test` of a false pass, and I could not reproduce that on a clean copy.

2. **What the brief could have handed me:**
   - That an option after `;` in a rule needs a default.
   - That helpers called from actions must be marked `sync`, or a test that recurses through them fails with "async recursion".
   - That `nikaia fmt` reflows code and uses tabs.
   - That `text::parse_f64` refuses `.5` and `1.` (ADR-285 D35).
   - That `std` has no radix parse. That one decided the `Result:` and could have been settled before the run.
   - That `ParseError`'s rendered message contains raw newlines inside the expected-token list. Reading line and column from that text is fragile, and I depend on it.
   - How to run one package test program on its own: the scratch project with a `path` dependency.

3. **Checks:** I ran `nikaia test --project packages/yaml` and `--bless` several times, ran single programs in the scratch project, then the `fmt::every_nika_file` scan, clippy and `pre-push-check` once each.
   - The clippy run checked no `crates/` change (the brief requires it before every push anyway).
   - The early `--bless` runs that had no expectation file yet produced nothing.
   - The scratch-copy check of the "false pass" proved nothing.

4. **The one change:** a *Known lowering defects and gaps for grammars* list in `builder-facts.md`, with each issue number: mixed-type group alternatives, `hex_digit`, `as scalar` from a number above 255, temporary `Bytes` arguments, no radix parse. With it, the brief could have said up front "0x/0o and `\x` escapes wait on #N, leave them out" instead of my finding that out mid-run. A second small fix: `nikaia test --bless` should write a `.stdout` for a test that has none.
