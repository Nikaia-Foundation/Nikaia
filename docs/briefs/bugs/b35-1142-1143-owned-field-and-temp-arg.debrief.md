1. **Most turns and time:** reproducing #1143. The brief said the probes are "verbatim in #1142 and #1143", but I have no GitHub access, so I wrote them from the one-line description. My first probe called `pad_left` inside `println` and passed. Only binding the result showed the failure. After the first build my test still failed, because it used an unannotated list, which the tiers pass cannot type. Second was the red `version` in `pre-push.nika`. I spent about five turns on it and regenerated the badge. It turned out `bump.py check` reads the shared working tree, not the staged copy.

2. **What the brief could have handed me:**
   - The two probes themselves, as code.
   - Which pass decides each fix: `check_core_stmt.nika`, where `lent_lets` and `moved_out` are set, for #1142, and `text_tiers.nika` (`method_kinds`, the `to_string` branch) for #1143.
   - The governing rules: ADR-094 D4, #665 and #962 (`let mut` over a field of a `mut` struct is a view), which I found through the spec and existing tests.
   - That the brief's test path `tests/compiler/NAME` really means `tests/compiler/tests/NAME`.

3. **Checks:**
   - Useful: both probes on the old binary; `lower` to read the Rust; `check --tethers`, which named the cause in one call; one build cycle; 26 touched programs in a scratch project; six related `it` modules; clippy on `nikaia-std --features compiler`; `pre-push`.
   - Wasted: regenerating the badge and re-running `pre-push` twice for a red that was never mine. A plain `git reset -q` I ran to unstage my files broke a rule; nothing was lost.

4. **One change:** a brief that cites an issue must paste its probe code in full, and say what the expected output depends on (here, a bound result and a declared element type). That would have saved most of the #1143 search. Also add to builder-facts: `pre-push`'s `version` check reads the working tree, so a red self-hosted badge there with `badges green` on the staged copy is not the builder's.
