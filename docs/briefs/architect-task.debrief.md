1. **Most cost:** reading `docs/architect.md`. At about 36k tokens it was over the read limit, mostly because of the long *State* log, so I had to read it in three pieces. After that, narrowing b1031's two findings to their causes took about 12 probe-and-read rounds: the lowered Rust, `kept_hulls`/`kept_owned`, `run_code` and the NK1131 path.
2. **Looked up, though it could have been handed to me:**
   - How to see the lowered Rust (`nikaia lower` writes `F.rs`), found in `builder-facts.md`.
   - Which Part I section has "a copy shares the one closure". I first cited 5.3 when it is 5.4, and had to fix both issues afterwards.
   - Where the capture handling lives (`emit_flow_kept`, `hulls_of_a_lambda`) and where #1031's `kept_fn_copies` is.
   - That `target/fast/nikaia` was older than 637bec45, so I rebuilt it before probing.
3. **Checks:**
   - Useful: the reduced probes a2, a3 and a4 (the defect needs no use after the lambda), the list-capture probe that showed the existing Arc/clone model, g (the parameter case has a separate cause), h (i64 return and `let g = h.f` pass), and running both test programs as they will be committed.
   - Wasted:
     - c.nika: `"abc".to_string()` hit NK1301, and the retried text capture lowered to `&str`, so it said nothing.
     - e.nika mixed four shapes and needed a follow-up probe to separate them.
     - The rebuild was needed, but one check of the binary's date against the head commit would have shown that before I probed.
   - Not possible: checking the proposed fixes themselves, since I don't build.
4. **One change:** split *State* out of `architect.md` into its own file, keeping only the last 3 entries in the charter. Also have `drive.nika` write the commit the binary was built from into `architect-input.txt`. That saves the oversized read and the doubt about a stale compiler on every architect run.
