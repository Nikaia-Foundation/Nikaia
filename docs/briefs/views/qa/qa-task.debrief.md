1. **Time sink.** Most of it went on the `result_source` test. I checked free functions, `keep`, `Vec[String]` and function-value variants one by one to see whether the NK2302 refusal was still reachable. I also lost a few turns to my own mistakes. A glob in `cd` failed, so `c.nika` landed in the repo root, and my grep patterns matched test names instead of failures. The two `cargo build` plus `lower-std` cycles were short, about 2 minutes each.

2. **Looked up that the brief could have handed me.**
   - The qa-input only said "arm64 bootstrap_compiler". The three failing test names were buried in `suite.log` around line 165.
   - I had to work out that the failing tests were not in v18a's listed test set, which skipped `lent_arguments`, `result_source` and `grammar_lowering`.
   - I had to find where `keeps_named_as_values` is and how `FoldSpec` and `Pattern` are shaped.
   - Which tools files share a namespace (private fns like `visited_in_maybe` worked across files).
   - How `commit.nika` handles several commits (it needs one report each).

3. **Checks.**
   - Useful: the three failing tests, the 332-test rerun, `lower-std` twice for the fixpoint, clippy and pre-push.
   - Wasted: probes with `$S/old/nikaia`, because that binary already contained v18a and could not show pre-commit behaviour. I could not learn from it why the old test refused.
   - I did not run the free-function `check` before deciding the test's meaning, which would have saved several probes.

4. **One change.** Have the driver's `qa-input.txt` list the failing test names and the file or line of the first panic, instead of a log tail. Add a builder.md rule: "when a rule changes what a method or function emits, grep `crates/nikaia/tests/it/` for the old behaviour and run those files". That would have caught all three failures in v18a before the suite did.
