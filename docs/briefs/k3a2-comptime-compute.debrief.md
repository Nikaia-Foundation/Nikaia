1. **What cost the most:** waiting on the `tests/compiler --both-settings` run (about 10 minutes, 398 programs), not turns. The one real design step was the Rust `ComputeHost`. Its units are numbered by position in a list, and the sub-program's unit is not next to the others in that list. So `driven` needed `lowered_units` to take `&[&Parsed]`, and `without_readers` needed a helper that works on units picked out by index. One `cargo check` found nothing more than two now-unused re-exports.
2. **What I looked up that the brief could have given me:**
   - The `Wall` variants (`InsideAnother { grammar }` maps to `name`).
   - The exact Rust signatures of the lowered `ComptimeComputeHost` (`&impl Fn` on the trait, `&dyn Fn` on the functions).
   - That `lowered_units` and `NOT_COMPUTED_YET` have no other callers, while `without_readers` is also called from `grammar_run.rs:336`.
   - That the K3a row of `docs/selfhosting-plan.md` already held another session's uncommitted note.
   - The path of the `rs2nika` binary for `--score`.
3. **Checks:**
   - Useful: `cargo check -p nikaia-std --features compiler` right after the first `lower-std` (green at once), `cargo check -p nikaia`, one build cycle with a stable fixpoint, the `it` filters next to clippy, the gate, `tests/compiler`, and `pre-push-check`.
   - Wasted: none really. I could have skipped `badges.py --check`, since `pre-push-check` covers it.
4. **One change:** give the brief a line on what the Rust host needs from code outside the cut. The arena's units are not next to each other in file order, so `lowered_units`/`without_readers` must take `&[&Parsed]` or `&mut [&mut Parsed]`. Also say that the consts' Rust re-exports become dead, and clippy refuses dead ones. A builder could then write the host in one pass without rereading the call sites.
   - Separately: when the brief orders the code taken from a patch, say whether `--score` is still expected to pass 60%. Here it measured how far the host-call shape is from the Rust, not what the converter is worth.
