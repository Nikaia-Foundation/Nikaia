1. **What cost the most:** the `match p { Props { a: x } => .. }` case. The brief's cause only covered the label's struct, so after the first build that case still failed. It took about four turns of probes, including two broken function-based probes that died on unrelated errors (`ref Props` result, `Bytes::text` can fail). Then I had to read `bind_a_payload`, and I placed my first edit after an early `return false`, which cost one more build. Bisecting the test's entry rules with a small script is what isolated the `match` case and the separate `p:(P)` group-label defect.

2. **What I looked up that the brief could have handed me:** where match patterns bind names (`bind_static_payloads` / `bind_a_payload`, and how the `Expr::Match` arm in `tier_expr` calls them). The brief listed the `match` case in the test but placed the cause only in `bind_grammar_pattern`. I also probed that a label on a group (`p:(P)`) is out of scope in the action; that is a separate defect the brief assumed works ("directly, or through a group").

3. **Checks:**
   - Useful: the unfixed-binary run of the test (it failed as expected); `nikaia check --tethers`, which showed `Pair.a` as mixed and pointed at the binding; the per-rule bisect; one background run of the gate, `tests/compiler` and clippy together; the yaml package gate.
   - Wasted:
     - The second build, whose edit could never run (the early return).
     - The two hand-written function probes.
     - The extra `cargo build` after each cycle "for the fixpoint", which was a no-op.
     - The `nikaia-std --lib text_tiers::` unit run, which matched 0 tests.

4. **The change that would save the most:** a brief whose test lists several shapes should say which shapes the architect actually traced, and mark the rest as unverified. For example: "the `match` case may have a separate cause in `bind_a_payload`; run the probe per shape before the first build." Generally, builder.md could say: bisect the probe into one program per shape on the old binary before editing, so separate causes show up before the first build.
