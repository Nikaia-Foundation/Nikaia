1. **What cost the most.** The one `nikaia test --project tests/compiler` run (about 10 minutes) and the build cycle. Waiting on the test run, I broke a rule: I wrote a `wait.sh` that looped on `sleep` until the output file showed an exit line. The background task would have woken me anyway, so that call was pointless. Three turns were lost to small slips:
   - `nikaia run` refused a file inside `tests/compiler` because it is not the project's entry, so I had to copy it to the scratchpad.
   - The test's `println(h.iter().sum())` failed at rustc. That is an old defect that also hits `Vec`; the probe that confirmed it cost one more turn.
   - My rustc check of the K3a form failed twice. I sent `nikaia lower` output into `k.rs`, which put its status line into the file. Then plain `rustc` could not build the file, because lowered code needs `nikaia_std`.

2. **What I looked up that the brief could have given me.**
   - That `std.contracts` is built into the binary, so a contract change needs a rebuild.
   - That the emitter already passes a method name like `iter` straight through to Rust, so only the contract was missing. I checked with a grep for `BTreeMap::iter`; the brief could have said "expect contract-only".
   - How to "compile by rustc" in practice. It means `nikaia run` on a scratch file, not plain `rustc`.

3. **Checks.**
   - Useful: the build cycle, the program probe, the Vec probe, `tests/compiler` once, the gate guard set, clippy, fmt and `pre-push-check`.
   - Wasted: the plain-rustc attempt and the sleep loop.
   - Probably extra: the throwaway probe in a tools file. It lowers through the same emitter as a program, and the `lower-std` fixpoint already showed the same thing. Clippy was also low value, since no Rust source changed, but it is required.

4. **The change that would save the most.** In builder.md, under the K3a-style probe step, say: "To check that lowered code compiles, run `nikaia run scratch.nika`. Plain `rustc` cannot link `nikaia_std`, and `nikaia lower` writes `F.rs` itself (do not redirect its stdout)." Also add a line saying a test file under `tests/compiler` is probed from a scratch copy.
