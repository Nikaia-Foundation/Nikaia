1. **What cost the most:** probing the language from scratch. I lost about six turns to syntax I guessed wrong: `var` instead of `let mut`, a bare `Map`, `use std::collections`, `.to_string()` on a literal (NK1301), `ref String` (NK1261), and `print` adding no newline. Separately, a `grep -r` over `crates/` timed out after 120 s and was moved to the background.

2. **What the brief could have handed me:**
   - **A probe template:** a ten-line `.nika` with the right `collections` and `print` forms.
   - **ADR pointers for `ref T` to `T`:** ADR-233 D4, ADR-279 D8/D9 and ADR-283 D3. I had to find them by reading, one at a time.
   - **Where `std` method signatures live:** `crates/nikaia-std/std.contracts`.
   - **The ADR index order:** that new rows go after 385 in the README.
   - **That `common.part` is the current rules block:** I first copied the longer, older block from the K3b brief and had to swap it.

3. **Checks run:**
   - **Useful:** the probes for map `drain`, the copy view and the C1c shape; `git merge-tree` for the K3b cherry-pick; `check-adr-refs.py`; `pre-push-check.sh`; the search of recent issues for b28's defect.
   - **Wasted:** the broad `grep -r` (it timed out), my first probe written in wrong syntax, and the `print` newline test.

4. **The one change that would save the most:** a *Probing* section in `docs/builder-facts.md`, also read by the architect. It would hold a correct probe skeleton, the binary path (`target/fast/nikaia`, which is not on PATH), and where `std` signatures and the ADR index live. That alone would have saved about eight turns.
