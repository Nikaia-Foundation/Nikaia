1. **What cost the most turns:** the 129-level depth test hung. The cause was a bug already on `develop`: the flow grammar re-read every nested key, so parse time grew exponentially. Finding it took probes from n=20 to n=26 and then a grammar rewrite (`FlowTail`). After that, rustc errors that `nikaia check` had passed cost several builds: the `Node?` guard parameter ("FnOnce"), and `peek("-" Gap)`. Last, `nikaia fmt` broke long rule lines before `*`/`+`, so I broke about ten lines by hand and checked fmt stayed stable each time.
2. **What I had to look up that the brief could have handed me:**
   - **Tags:** the brief says "this brief resolves them", but ADR-yaml-1 D6 says the tree keeps tags and `load` resolves them. I had to settle that from the ADR myself.
   - **YAML 1.2.2 cases:** the reference cases for each construct (6.19 for `!!` redefined, XLQ9, 2JQS).
   - **Grammar syntax:** how cut, `peek` and an optional label behind a guard are spelled.
   - **The old diff:** sorting which parts of the earlier attempt's diff held up.
3. **Checks run:**
   - **Useful:** `nikaia test --project packages/yaml` (several runs), `fmt::every_nika_file`, `pre-push-check.sh`, and the small n-level flow probes.
   - **Wasted:**
     - one test round against the old diff's unblessed `.stdout` files (`directives` and `explicit_keys` failed);
     - repeated full package runs where a single program would have done;
     - `commit.nika --dry-run`, which showed the script would paste the "Earlier attempt" section into the message, so I committed by hand.
   - **Not run:** the gate script, because nothing under `crates/` changed. The report's `Gate: green` means the brief's package gate only.
4. **The change that would save the most:** keep the `Attribution:` block the last section of a brief, with nothing after it, so `commit.nika` can be used and the push is not delayed until `develop` moves. Close behind: note in the brief that the flow grammar on `develop` is exponential in nesting, since the depth test cannot run until that is fixed. Also make the brief agree with ADR D6 on tags: kept as written, resolved only by `load`.
