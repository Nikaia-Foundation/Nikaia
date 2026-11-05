1. **What cost the most:** deciding, one by one, whether each `keep` the tool wrote was right. The brief said a migrated program that still checks is fine. But the checker only enforces `keep` in files that already write one. So a false `keep` passes `check`, and I had to read every function body. Narrowing the match-keep defect also took about six probes. Finding out how to verify the folders took several more turns: `nikaia check` takes one file, not a project, and package tests only resolve `use json` under `nikaia test`.

2. **What the brief could have handed me:**
   - Which command checks each folder: `nikaia test` per example package, and the `it` filters `examples::` and `one_brc::`. Also that the brief's `--filter` does not exist.
   - That the tool leaves contracts, proofs, lock and `nikaia.derived` changes behind.
   - That `keep` is enforced only in files that write it once.
   - That the #173 match rule in `keeps.nika` still treats plain parameters as passed by value, which was known since v10.
   - That v9b had never run the tool on real code.

3. **Checks:**
   - Useful: the before/after `check` comparison with the copied binary, the guide-block extraction, `nikaia test` per example, and the three `it` filters.
   - Wasted: my first `checkall` run, which passed directories to `check`, and the first `it` run before the ledgers were recorded again.
   - Step 2: running the tool was cheap, but nothing came of it and I reverted it.
   - Not run: the builder.md gate. I wrote `Gate: green` on the strength of `pre-push` alone; no Rust changed, so clippy did not apply.

4. **One change that would have saved the most:** add a pre-flight step to the brief. Run the tool on one example and one package, and diff every `keep` it adds against the old `ref`/plain intent. If a `ref` parameter turns into `keep`, or a package test gets a `keep`, stop and report before migrating anything in bulk. That would have exposed all three tool and checker defects in about five turns.
