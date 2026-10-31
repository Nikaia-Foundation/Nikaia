1. **Most cost:** my own output filter. `grep -E '^test result'` also matches test names starting with `result_source`, so I ran the same `it` test set three times, about two minutes of tests wasted. After that came the background run of `tests/compiler --both-settings` plus `packages/yaml`, which was necessary.
2. **Looked up:** where a declared type's fields can be read. That is `Unit.fields`, filled by `view_fields`, with an enum's variants flattened into it, so a field-less enum gives an empty list. I also checked that `unit` is in scope in `rsrc_function`. The brief's *Locations* pointed at `rsrc_ty_holds_view`, which works from the ledgers and doesn't help here, because a struct holding an owned `String` is not tethered. Naming `unit.fields` instead would have saved two reads.
3. **Checks run:**
   - the probes on the old binary and the fixed one (three shapes);
   - the new test against the unfixed suite binary;
   - one build cycle (fixpoint clean);
   - the `it` NK2302 modules plus `accepted` and `fmt::every_nika_file`;
   - the `nikaia-std` unit tests;
   - `tests/compiler --both-settings` and `packages/yaml`;
   - `cargo fmt`, clippy on Rust 1.97, `pre-push-check.sh`.

   Wasted: the two `it` re-runs. I skipped clippy on CI's Rust 1.99.1 for lack of disk (7.3 GB free), which is a gap rather than a saving. I listed it under `Tests:`, but builder-facts asks for that run when Rust changes.
4. **Change that would save the most:** builder.md should give the exact filter for cargo test output, `grep -E '^test result:|\.\.\. FAILED|^---- '`, with the colon. It should also say whether a change only to `lowered/*.rs` needs the newer-clippy run, or that clippy on `-p nikaia-std --features compiler` alone is enough. That would make the check affordable on a full disk.
