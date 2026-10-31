1. **What cost the most.** I built before testing the brief's claim that the seed in `keeps.rs` was the only thing keeping data parameters. The full cycle (build, then `lower-std`) came back empty, and only then did I start tracing. Stuck on the empty result, I went through `nikaia run`, then `nikaia lower` plus the `nikaia.contracts` file, then grepped the NK1102 text into `check_arguments.nika`, then `lends.nika` and `emit_core_args.nika`. That was about 8 turns of tracing that one probe would have saved.

2. **What I had to look up.** Four things the brief could have handed me:
   - that `lends_with` and `emit_arg_only_read` never lend a data parameter of any method that takes a receiver;
   - that `FnContract` has no flag saying a method belongs to a trait;
   - the D17 bullet *every by-value parameter* (I read the ADR to see whether methods are covered);
   - how to dump the ledger for a probe (`nikaia lower F.nika` writes `nikaia.contracts`; `nikaia run` writes nothing I could inspect).

3. **Checks.**
   - Useful: running the test program on the old binary (it showed NK2105 and NK1102), the ledger dump of a small probe, and the inherent-method probe.
   - Wasted: the `cargo build` and `lower-std` of the seed removal, the confused re-check that it really had built, and the rebuild to restore `develop`'s binary. That is three builds of about 40 s each for no information.
   - Not run, correctly: the gate, clippy, `it` and `tests/compiler`.

4. **The one change that would have saved the most.** builder.md, before the first build: *When a brief says "remove X and the signatures follow", first lower a probe with the old binary and find the code that decides the lowered type (`grep lends_in`, `takes_a_receiver`). If the column you change does not reach that code, stop.* The brief itself should have carried one verified probe line showing the claimed cause, for example: an inherent method with `text: String` already lowers to `&str`.
