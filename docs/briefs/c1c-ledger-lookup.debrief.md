1. **Most time:** two things, about equally.
   - Two builds failed because my doc-comment splice was off by a line at each end.
   - The real cost was the lifetime refusal, which I only found after a full build. I then spent a few turns on `--tethers` and `tether.nika` and concluded it needed an architect decision.

2. **What the brief could have handed me:**
   - The brief's "Probe" only tested `lookup` inside Nikaia. It never ran the Rust forward, which is where the failure appeared.
   - It could have said that a `name: String` parameter next to a `ref Ledger` gets one shared `'r` lifetime.
   - It could have given the exact line range of the doc comment (523-541), not just ":523-541" next to a Cut of 542-546.

3. **Checks:**
   - Useful: the first `lower-std` (it caught the syntax slip in seconds), `cargo build` (it showed the E0308 and then the lifetime error), and `--tethers` on a copy of the probe.
   - Wasted: nothing large.
   - Not run: the contracts and `it` tests, because the build never got that far. I did not compute the `rs2nika --score`.

4. **One change:** the brief's probe should include the Rust-side forward, a tiny `fn f(&self, name: &str) -> Option<(String, &T)>` calling the lowered function. It would have shown before any build that two-lifetime signatures lower to a single `'r`. That would have turned this into a "blocked, needs lowering fix" brief from the start.
