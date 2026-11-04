1. **What cost the most:** the build cycles. A `comptime` `ref String` note was refused as `Vec[ref String]`, and clippy flagged a nested `if` in the lowered output. Each one meant another `lower-std` and build. The second cycle ran past 10 minutes and went to the background. The interrupted first run also cost some re-orientation.
2. **What I had to look up:**
   - where NK1261 is decided (`types.nika`, not `check_core_places.nika`, as the brief guessed);
   - that the checker compares `Ty::Fn` from `declared.nika`, so names in the AST `Code` don't change the type;
   - the parser's `code_param` rule and `result_with_from`;
   - that a trait with an owned `String` parameter and a `from` fails to lower (I needed a `ref` parameter to get a runnable test);
   - that a test with warnings needs both `.stdout` and `.stderr`, and that `--bless` only writes files that already exist.
3. **Checks I ran:** probes on the old binary (useful, they showed both defects before any build), the `lower-std`/build cycle three times, a scratch project with `--both-settings --bless` (three runs: two blessed, one checked), parser unit tests, clippy, `pre-push`. Wasted: the first bless run, which wrote no expectation files for the new tests (I hadn't created placeholders), and the `fmt --check` on scratch copies, which tells nothing because files outside the project are formatted with a different style.
4. **One change that would have saved the most:** have the brief say where the check lives (`types.nika`, `item_types` / `written_at_the_boundary`), that a `Vec[String]` note must be built from `String`s, not `ref String` constants, and that a new output test needs empty `.stdout`/`.stderr` placeholders before `--bless`. Together that would have saved about two build cycles and one test run.
