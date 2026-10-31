1. **What cost the most:** the named-variant shape. The brief named "tuple and named variant patterns" but gave no probe for the named one. It still refused after my first build, and only then did I find that `declared_of` skips named variants entirely. That cost a second build cycle of about 3 turns. The 10-minute `tests/compiler` run was the longest wait, but it was necessary. Writing the `for` probe with a bare `=> for x in ..` arm also cost one turn (`for` is a reserved word there).
2. **What I had to look up:**
   - How #1063 (ac156203) made `for` elements work: `TierLocal.of` through `walked_struct`/`element_struct`. The brief could have named `TierLocal.of` and `struct_of_local` as the mechanism to copy.
   - That `TextDeclared.variants` holds only tuple variants.
   - Whether `bind_static_payloads` runs for every arm (it does).

   All were quick greps; the brief's Locations were accurate.
3. **Checks:**
   - Run: the probes on the old binary, then the new one; the unfixed suite binary for the before-fix check; one `lower-std`/build cycle per change; `tests/compiler --both-settings`; the `packages/yaml` tests; `fmt::every_nika_file`; `cargo fmt`; clippy; `pre-push-check`; and `cargo test -p nikaia-std --lib text_tiers::`.
   - Wasted: the unit-test run, which matched 0 tests because there are no lib tests for a `.nika` module. Probably also the separate `nikaia fmt` diff pass, which caught one trailing comma.
   - Skipped: clippy 1.99.1, because it isn't installed and needs a cold build.
4. **One change:** a brief that names a pattern shape should carry a probe for each shape it names. Here that means the named-variant probe, plus a note on which of them go through declared positions (`TextDeclared`). Builder.md could also say "probe every shape the brief names on the old binary first", which it nearly does already, but a ready-made probe per shape would have saved the second cycle.
