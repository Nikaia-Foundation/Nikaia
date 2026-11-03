1. **Most turns:** working out which parts of the brief v1 had already done. Part I 6.5 had already dropped "returns it by value", Part I already cited `NK1333`–`NK1336`, and `NK2302` already had the wording step 2 asked for. That took about five turns of grep and reads. An off-by-one in a Python line-index edit cost one more turn.
2. **Looked up that the brief could have handed me:**
   - how `tests/specification` is run: `cargo test -p nikaia --test it -- specification::`, in `crates/nikaia/tests/it/specification.rs`;
   - the format of the ADR README's "Changed by" table and index rows;
   - whether any registry of NK codes must match Part III (there is none, which cost one wasted grep);
   - where `windows`/`chunks` and the byte buffer sit in Part III (line 382);
   - that the brief's "grammar inputs" use `Bytes` and contain no `ref Array`.
3. **Checks I ran:** `specification::` twice (25 s each), `pre-push` twice, and the cache and disk check. Wasted: the grep for a code registry and the check that `NK1137`/`NK1158` exist.
   - `Gate: green` is not backed by a gate run. The brief orders the gate, while builder.md says the driver runs it, and no Rust changed, so I ran neither `cargo fmt` nor clippy. The supervisor should know that the field means only my cheap checks.
4. **One change:** a "State after v1" line in the brief listing what v1 already wrote (6.5 list, the Part I codes, `NK2302`), plus the exact test command. That would have saved about eight turns. Also settle in builder.md what `Gate:` means for a builder, now that the driver runs the gate.
