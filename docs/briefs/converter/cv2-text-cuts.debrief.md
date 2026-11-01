1. **What cost the most.** An emitter defect I didn't expect. Adding `text_expr` created the call cycle `expr → text_expr → field_ty → ty → expr`, which runs through two files of rs2nika. rustc then refused the build with "recursion in an async fn requires boxing". The cause was that `pausing_reach` only looked at the one file being lowered. Isolating it took a two-file probe and a check that joining the files made it pass. Fixing it took a change in `emit_reach.nika` and the Rust adapter, then `lower-std` and its fixpoint, and a new `tests/compiler` test with a scratch project to show it failing before the fix. That was about half the run, all outside the converter.
2. **What I looked up that the brief could have handed me:**
   - Where the converter learns a name's type. `self.texts` and `shared_texts` only cover parameters, so I had to write my own lookup for local bindings.
   - What `field_ty` returns.
   - What `self.fails` means.
   - That rs2nika's packages are lowered file by file with `beside`.
   - That a test's `.stdout` must be written with stderr dropped.
   - Which of the convert package's `nikaia.contracts`, `.derived`, `.proofs` and `.lock` files were already dirty from other agents.
3. **Checks I ran:**
   - Useful: `nikaia check` of each target form, and of the S1 draft on a scratch copy; the convert package's `nikaia test`; the old-against-new probes for the cycle; `cargo test --lib emit::`; clippy; fmt; `pre-push-check`.
   - Wasted:
     - A second clippy run on rustc 1.99.1, with its own target directory that I then deleted.
     - The first `text_cuts` test run, which failed only because stderr got into the `.stdout` file.
     - Re-running the corpus score. The brief already implied it wouldn't move, and it didn't.
4. **The one change that would have saved the most:** a sentence in `builder.md` / builder-facts. It would say that the converter is a package compiled file by file, so a new mutual call between its files can hit compiler gaps that only show across files. It should name `pausing_reach` / `code_on_a_cycle` as the known spot, and give the two-file probe as the first thing to try. The brief could also name the helpers that already exist (`texts`, `field_ty`, `fails`) next to the rule.
