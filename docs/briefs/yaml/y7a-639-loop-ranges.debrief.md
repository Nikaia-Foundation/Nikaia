1. **Most costly:** finding out why loops of 7+ turns were not proved. It took about 8 turns: the 7/8/16-turn tests, reading `logic_search`/`logic_lia`, a throwaway `eprintln!` in `proofs.rs` (two Rust builds), then reverting it. Second was the `sequences` guard. I changed `.iter().all` to a `for`, rebuilt and re-ran, and only then found it already reads 18 on develop. That wasted one build and two runs of about 35 s each.
2. **Looked up, could have been handed over:**
   - The spike only proves loops of 6 turns or fewer: past that, the final check runs out of the solver's 1024 cases.
   - `lg_loop_check` is called in two places (`loop_found` and `loop_recorded`).
   - A test program can't set `unproved-claims = "warn"`, so the 17-turn test has to be a `.stderr` refusal.
   - `0..x` fails `range_spelling` (write `0..<(x + 1)`).
   - `nikaia check` run inside `tests/compiler` checks the whole project, so probes go in a scratch copy.
   - There is no switch that prints why the solver answered *unknown*.
3. **Checks:** the cycle 3×, probes against the old and new binary, `it -- prove unproved_claims` plus the guard set 2×, clippy 2×, `tests/compiler` both settings 1×, `pre-push-check` 2×.
   - Wasted: the second guard-set run and the rebuild that only removed `.iter().all` (both caused by the guard that was already red).
   - The spike's 4-turn probes alone would have missed the 16-turn limit. Running the turn count as a range of values (4, 6, 7, 16, 17) caught it.
4. **One change:** builder.md should keep a line, refreshed by whoever pushes, saying which guard tests are red on develop right now and at what count (for example "sequences: 18/16 since 7c0ff303"). That would also let `Gate:` say "red, not mine" without breaking the rule. For prover briefs, also: "probe at the claimed limit (16 turns), not only the spike's example". The spike's 4-turn result hid the budget failure.

The report file changed on disk after my last edit; I did not re-read it.
