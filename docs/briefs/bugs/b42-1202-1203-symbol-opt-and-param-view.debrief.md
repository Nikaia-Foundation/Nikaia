1. **Most turns and time:** the full `tests/compiler` run (about 10 minutes), then chasing its 7 failures. Six came from another agent's rebuild deleting `target/fast/nikaia` mid-run. When I reran them alone, they failed on both binaries, and that was my own fault: I had named my copies `nikaia-old`/`nikaia-new`. The script tests only accept a running binary named exactly `nikaia`; otherwise they fall back to the relative `target/fast/nikaia`, which doesn't exist in a scratch project. That cost three extra runs.

2. **Lookups the brief could have handed me:**
   - **The issue texts:** I fetched them with `gh api` (REST), so the brief should paste the probe and the *To build* list.
   - **Where the `Some` wrapping is decided:** `check_function_wrap_tail` / `check_widened_slot` set a wrap of the result, and an unknown type gets no `Some`. The brief could have named this.
   - **Where loops are walked:** the loop arms of `tier_statement` in `text_tiers.nika`.
   - **The words table:** how a grammar entry takes it (ADR-388, the `grammar_words_required_kept` test).

3. **Checks run:**
   - Probes on the old binary and several shape variants with `--tethers`. These were the most useful: the `d` variant showed the real cause was the loop's back edge, not "the first initializer".
   - One build plus the fixpoint check, both new tests old vs new, the full `tests/compiler`, 203 `it` tests, clippy, fmt, `pre-push`.
   - Wasted: the first alone rerun of the 7, both binaries, with the wrongly named copies. The full `tests/compiler` run was arguably more than "the touched tests", but the tier change affects every loop.

4. **One change:** in the builder-facts entry *Test with your own copy of the binary*, add: "keep the copy in its own directory and name it exactly `nikaia` (`$S/new/nikaia`); the script tests check the basename and otherwise fall back to `target/fast/nikaia`."
