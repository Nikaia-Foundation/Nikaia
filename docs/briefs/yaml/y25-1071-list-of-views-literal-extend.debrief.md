1. **What cost the most.** Mostly waiting on the `tests/compiler --both-settings` run plus the yaml package tests, which were one background call. The only wasted work was a probe round. My first two negative probes (`neg`, `neg2`) passed on the old binary too, so they proved nothing and I had to write four more. One reason was that I guessed wrong which refusal applies.
2. **What the brief could have handed me.**
   - The brief's own example for step 2 ("a view of a by-value parameter, `NK2302`") does not hold. On the old binary, a by-value `String` viewed into a pushed element is accepted. With two `ref` parameters the refusal is `NK2106`, not `NK2302`.
   - A negative probe the architect had checked as refused today would have saved that round.
   - The brief's *Locations* were exact, so I had nothing to search for.
3. **Checks.**
   - Useful: the old-binary probe matrix, one build cycle that needed no second fixpoint build, the new-binary probes, `tests/compiler --both-settings`, `packages/yaml`, `fmt::every_nika_file`, and `pre-push-check`.
   - Wasted: the first `pre-push-check`, piped to `tail`, printed nothing, so I ran it again to get the exit code. Also the two negative probes that were never refused.
   - Skipped on purpose: clippy and `cargo fmt`, because no Rust changed.
4. **One change.** In the brief template, give every "stays refused" step a probe that fails on today's binary, with the code it fails with. Or have builder.md say: "a negative probe must be refused on the unfixed binary first, or it proves nothing."

Also, a correction to my report: I wrote `Gate: green` for the checks I ran, but the full suite is the driver's. The disk figures (3.7 GB cache, 7.5 GB free) are only in my final message, not in the report's lines.
